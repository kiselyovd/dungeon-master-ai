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
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            self.checkpoint_with_retries(),
        )
        .await
        .unwrap_or(Err(
            crate::ports::repositories::RepositoryError::Operation {
                operation: "room_agent",
                code: "room_agent_busy",
            },
        ))
    }
    async fn checkpoint_with_retries(
        &self,
    ) -> Result<
        Vec<crate::models::campaign::StoredMessage>,
        crate::ports::repositories::RepositoryError,
    > {
        let mut attempts = 0_u64;
        loop {
            match self
                .rooms
                .rooms
                .room_agent_checkpoint(self.subject.lease(), self.turn_id)
                .await
            {
                Err(crate::ports::repositories::RepositoryError::Operation {
                    code: "room_agent_busy",
                    ..
                }) if attempts < 6 => {
                    attempts += 1;
                    tracing::debug!(
                        attempt = attempts,
                        "retrying locked room checkpoint in a fresh transaction"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(10 * attempts)).await;
                }
                result => return result,
            }
        }
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
        if let Some(saved) = self
            .rooms
            .rooms
            .room_dm_request(self.subject.lease(), self.turn_id)
            .await
            .map_err(super::characters::repository_error)?
        {
            if saved.request.player_message != player_message {
                return Err(super::RoomError::Conflict);
            }
            return Ok(crate::agent::turn::AgentTurnCommand {
                request_id: self.turn_id.to_string(),
                request: saved.request,
            });
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
                "adventure_action" | "combat_action" | "rest_action" | "roll_dice"
            )
        })
        .collect::<Vec<_>>();
    for tool in &mut tools {
        if tool.name == "roll_dice" {
            tool.description = "Resolve a skill check requested by the current human player. Use dice 1d20 and check {character_id, skill, dc}; omit modifier. Only the initiating player's hero may be checked. The engine derives modifiers, armor penalties and records dice for replay. This check does not choose or execute a hero action.".into();
            tool.parameters["required"] = serde_json::json!(["dice", "check"]);
        }
    }
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
        prompt.push_str("\nYou are the Dungeon Master of this shared campaign, never a player character. Lead narration, portray NPCs and adjudicate scenes and combat. Every hero is controlled exclusively by its human player. Resolve only the initiating player's declared hero intention; do not choose a hero's decisions, dialogue, movement, attacks or rest on their behalf. Ask the player when their intention is ambiguous. The deterministic engine owns dice, damage, resources and NPC combat turns. Describe consequences only after confirmed tool results. Accepted hero biographies and hidden canon are DM knowledge, not automatically public discoveries; reveal them only when the adventure establishes discovery. Other heroes are not controllable through the initiating player's tools. For a skill check use roll_dice with dice 1d20 and check {character_id, skill, dc}, using only the initiating player's accepted character ID; omit modifier. The server derives proficiency and armor/condition penalties and records the outcome. A check does not itself perform Hide, Search or any other hero action.");
        let mut dm_party = self
            .rooms
            .rooms
            .room_dm_party(self.subject.lease())
            .await
            .map_err(|_| AgentContextError {
                code: "room_agent_denied",
            })?;
        for character in dm_party.characters.values_mut() {
            if let Some(document) = character.as_object_mut() {
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
        }
        prompt.push_str(&serde_json::to_string(&serde_json::json!({"initiating_player_member_id":self.subject.lease().member_id(),"dm_party":dm_party,"combat":state.combat,"scene":state.scene,"clock":state.clock})).map_err(|_|AgentContextError {code:"room_agent_context_invalid"})?);
        if let Some(adventure) = &state.adventure {
            prompt.push_str(&crate::campaigns::adventure_context::bell_dm_context(
                &adventure.state,
            ));
        }
        let combat = state
            .combat
            .as_ref()
            .map(|combat| serde_json::to_value(combat).expect("typed combat"));
        let public_intention_context = crate::agent::context::public_intention_context(
            state.adventure.as_ref(),
            combat.as_ref(),
            state
                .own_character
                .as_ref()
                .and_then(|character| character["name"].as_str()),
        );
        Ok(AgentContext {
            reasoning_is_private: true,
            adventure_active: state.adventure.is_some(),
            system_prompt: prompt,
            combat_active: state.combat.is_some(),
            public_intention_context,
        })
    }
}
#[async_trait::async_trait]
impl ToolDispatcher for RoomAgentPorts {
    async fn replay(
        &self,
        request: ToolDispatchRequest,
    ) -> Result<Option<ToolExecution>, ToolDispatchError> {
        if request.request_id != self.turn_id.to_string()
            || request.campaign_id != self.subject.lease().campaign_id().0
            || request.session_id != self.subject.lease().session_id().0
        {
            return Err(ToolDispatchError {
                code: "room_agent_denied",
            });
        }
        self.rooms
            .rooms
            .replay_room_agent_tool(
                self.subject.lease(),
                self.turn_id,
                &request.tool_call_id,
                request.command,
            )
            .await
            .map_err(|_| ToolDispatchError {
                code: "room_agent_replay_failed",
            })
    }
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
                | AgentToolCommand::RollDice(_)
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
            AgentToolCommand::RollDice(command) => self
                .rooms
                .rooms
                .resolve_room_skill_check(
                    self.subject.lease(),
                    self.turn_id,
                    &request.tool_call_id,
                    command,
                    uuid::Uuid::new_v4().as_u128() as u64,
                )
                .await
                .map_err(|error| match error {
                    crate::ports::repositories::RepositoryError::Operation {
                        code: "room_access_denied" | "hero_check_incapacitated",
                        ..
                    } => super::RoomError::Denied,
                    _ => super::RoomError::Repository,
                }),
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
        if matches!(
            result,
            Err(super::RoomError::Repository | super::RoomError::Secrets)
        ) {
            return Err(ToolDispatchError {
                code: "room_intention_uncertain",
            });
        }
        Ok(match result {
            Ok(result) => ToolExecution {
                result,
                is_error: false,
                handled_by: "engine".into(),
                media: vec![],
            },
            Err(_) => ToolExecution {
                result: serde_json::json!({"error":"room_intention_rejected","resolution":"rejected"}),
                is_error: true,
                handled_by: "engine".into(),
                media: vec![],
            },
        })
    }
}
