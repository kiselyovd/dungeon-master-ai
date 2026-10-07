use super::{RoomService, RoomSubject};
use crate::agent::{
    context::{AgentContext, AgentContextError, AgentContextPort, AgentContextRequest},
    tool_dispatch::{ToolDispatchError, ToolDispatchRequest, ToolDispatcher, ToolExecution},
};
use std::sync::Arc;
use uuid::Uuid;

pub struct RoomAgentPorts {
    pub(crate) rooms: Arc<RoomService>,
    pub(crate) subject: RoomSubject,
    pub(crate) turn_id: Uuid,
}
impl RoomAgentPorts {
    pub fn new(rooms: Arc<RoomService>, subject: RoomSubject, turn_id: Uuid) -> Self {
        Self {
            rooms,
            subject,
            turn_id,
        }
    }
    pub async fn checkpoint(
        &self,
    ) -> Result<
        Vec<crate::models::campaign::StoredMessage>,
        crate::ports::repositories::RepositoryError,
    > {
        self.rooms
            .rooms
            .room_agent_checkpoint(self.subject.lease(), self.turn_id)
            .await
    }
    pub async fn command(
        &self,
        player_message: String,
    ) -> Result<crate::agent::turn::AgentTurnCommand, super::RoomError> {
        use crate::models::chat::ChatMessage;
        if player_message.trim().is_empty() || player_message.len() > 16384 || self.turn_id.is_nil()
        {
            return Err(super::RoomError::InvalidInput);
        }
        let saved = self
            .checkpoint()
            .await
            .map_err(super::characters::repository_error)?;
        if saved
            .first()
            .is_some_and(|first| first.message != ChatMessage::user_text(&player_message))
        {
            return Err(super::RoomError::Conflict);
        }
        let state = self.rooms.initial_updates(&self.subject, None).await?.state;
        if state.own_character.is_none() {
            return Err(super::RoomError::Denied);
        }
        let own_ids = saved
            .iter()
            .map(|message| message.id)
            .collect::<std::collections::BTreeSet<_>>();
        let history = state
            .messages
            .into_iter()
            .filter(|message| !own_ids.contains(&message.id))
            .map(|message| {
                if message.role == "user" {
                    ChatMessage::user_text(message.text)
                } else {
                    ChatMessage::Assistant {
                        content: message.text,
                    }
                }
            })
            .collect();
        Ok(crate::agent::turn::AgentTurnCommand {
            request_id: self.turn_id.to_string(),
            request: crate::models::agent::AgentTurnRequest {
                campaign_id: state.room.campaign_id.0,
                session_id: state.room.session_id.0,
                player_message,
                history,
                images: vec![],
                board: None,
            },
        })
    }
}
pub fn scoped_catalog(
    mut catalog: crate::agent::tools::AgentToolCatalog,
) -> crate::agent::tools::AgentToolCatalog {
    // A committed combat intention may need receipt recovery after combat ended.
    // Availability is enforced by the authoritative room dispatcher, not a transient phase.
    let mut tools = catalog
        .exploration
        .into_iter()
        .chain(catalog.combat)
        .filter(|tool| {
            matches!(
                tool.name.as_str(),
                "adventure_action" | "combat_action" | "rest_action"
            )
        })
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| left.name.cmp(&right.name));
    tools.dedup_by(|left, right| left.name == right.name);
    catalog.exploration = tools.clone();
    catalog.combat = tools;
    catalog
}
#[async_trait::async_trait]
impl crate::ports::repositories::MessageRepository for RoomAgentPorts {
    async fn append(
        &self,
        session: Uuid,
        message: crate::models::chat::ChatMessage,
    ) -> Result<crate::models::campaign::StoredMessage, crate::ports::repositories::RepositoryError>
    {
        if session != self.subject.lease().session_id().0 {
            return Err(crate::ports::repositories::RepositoryError::Operation {
                operation: "room_agent",
                code: "room_access_denied",
            });
        }
        self.rooms
            .rooms
            .append_room_agent_message(self.subject.lease(), self.turn_id, message)
            .await
    }
    async fn list(
        &self,
        session: Uuid,
    ) -> Result<
        Vec<crate::models::campaign::StoredMessage>,
        crate::ports::repositories::RepositoryError,
    > {
        if session != self.subject.lease().session_id().0 {
            return Err(crate::ports::repositories::RepositoryError::Operation {
                operation: "room_agent",
                code: "room_access_denied",
            });
        }
        self.checkpoint().await
    }
}
#[async_trait::async_trait]
impl AgentContextPort for RoomAgentPorts {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError> {
        let state = self
            .rooms
            .initial_updates(&self.subject, None)
            .await
            .map_err(|_| AgentContextError {
                code: "room_agent_denied",
            })?
            .state;
        if request.campaign_id != state.room.campaign_id.0 || state.own_character.is_none() {
            return Err(AgentContextError {
                code: "room_agent_denied",
            });
        }
        let mut prompt = request.base_system_prompt;
        prompt.push_str("\nThis is a shared campaign. Act only for the initiating member's accepted hero. Other heroes are public summaries, not controllable actors. Use only confirmed engine intentions, do not assign damage/resources in prose. Do not reveal private context or hidden NPC knowledge as player facts.");
        let mut own = state.own_character.clone().ok_or(AgentContextError {
            code: "room_agent_denied",
        })?;
        if let Some(document) = own.as_object_mut() {
            document.remove("portraitUrl");
            for key in [
                "backstory",
                "personality",
                "ideals",
                "bonds",
                "flaws",
                "notes",
            ] {
                if let Some(serde_json::Value::String(text)) = document.get_mut(key) {
                    *text = text.chars().take(4096).collect();
                }
            }
        }
        prompt.push_str(&serde_json::to_string(&serde_json::json!({"own_character":own,"party":state.actors,"combat":state.combat,"scene":state.scene,"clock":state.clock})).map_err(|_|AgentContextError {code:"room_agent_context_invalid"})?);
        if let Some(adventure) = &state.adventure {
            prompt.push_str(&crate::campaigns::adventure_context::bell_dm_context(
                &adventure.state,
            ));
        }
        Ok(AgentContext {
            adventure_active: state.adventure.is_some(),
            system_prompt: prompt,
            combat_active: state.combat.is_some(),
        })
    }
}
#[async_trait::async_trait]
impl ToolDispatcher for RoomAgentPorts {
    async fn execute(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<ToolExecution, ToolDispatchError> {
        use crate::{
            agent::commands::AgentToolCommand,
            models::{
                identity::Revision,
                room_combat::RoomCombatCommand,
                room_world::{RoomAdventureCommand, RoomRestCommand},
            },
        };
        let state = self
            .rooms
            .initial_updates(&self.subject, None)
            .await
            .map_err(|_| ToolDispatchError {
                code: "room_agent_denied",
            })?
            .state;
        if request.request_id != self.turn_id.to_string()
            || request.campaign_id != state.room.campaign_id.0
            || request.session_id != state.room.session_id.0
        {
            return Err(ToolDispatchError {
                code: "room_agent_denied",
            });
        }
        if !matches!(
            request.command,
            AgentToolCommand::AdventureAction(_)
                | AgentToolCommand::CombatAction(_)
                | AgentToolCommand::RestAction(_)
        ) {
            return Ok(ToolExecution {
                result: serde_json::json!({"error":"room_tool_unavailable"}),
                is_error: true,
                handled_by: "engine".into(),
                media: vec![],
            });
        }
        let prepared = self
            .rooms
            .rooms
            .prepare_room_agent_tool(
                self.subject.lease(),
                self.turn_id,
                &request.tool_call_id,
                request.command.clone(),
            )
            .await
            .map_err(|_| ToolDispatchError {
                code: "room_agent_tool_conflict",
            })?;
        let result = match request.command {
            AgentToolCommand::AdventureAction(command) => self
                .rooms
                .adventure(
                    &self.subject,
                    RoomAdventureCommand {
                        request_id: prepared.nonce,
                        expected_revision: Revision(prepared.expected_revision),
                        action: command.action,
                    },
                )
                .await
                .map(|result| serde_json::to_value(result).expect("typed receipt")),
            AgentToolCommand::RestAction(intent) => self
                .rooms
                .rest(
                    &self.subject,
                    RoomRestCommand {
                        request_id: prepared.nonce,
                        expected_revision: Revision(prepared.expected_revision),
                        intent,
                    },
                )
                .await
                .map(|result| serde_json::to_value(result).expect("typed receipt")),
            AgentToolCommand::CombatAction(action) => self
                .rooms
                .act_combat(
                    &self.subject,
                    prepared.encounter_id.ok_or(ToolDispatchError {
                        code: "room_agent_combat_missing",
                    })?,
                    RoomCombatCommand {
                        request_id: prepared.nonce,
                        expected_revision: prepared.expected_revision,
                        action,
                    },
                )
                .await
                .map(|result| serde_json::to_value(result).expect("typed combat result")),
            _ => unreachable!(),
        };
        Ok(match result {
            Ok(result) => ToolExecution {
                result,
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            },
            Err(_) => ToolExecution {
                result: serde_json::json!({"error":"room_intention_rejected"}),
                is_error: true,
                handled_by: "engine".into(),
                media: vec![],
            },
        })
    }
}
