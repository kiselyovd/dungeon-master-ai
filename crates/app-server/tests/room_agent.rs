use adapter_secrets::InMemorySecretsStore;
use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    agent::{
        commands::{AdventureActionCommand, AgentToolCommand, ApplyHealingCommand},
        context::{AgentContextPort, AgentContextRequest},
        tool_dispatch::{ToolDispatchRequest, ToolDispatcher},
    },
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        room::CreateLocalRoom,
        room_character::{ApproveRoomCharacter, SubmitRoomCharacter},
        session::CampaignSeed,
    },
    ports::repositories::{
        CampaignStateRepository, MessageRepository, PartyRepository, SnapshotRepository,
    },
    rooms::{agent::RoomAgentPorts, JoinRoom, RoomSecret, RoomService, SystemRoomClock},
};
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
struct Provider(std::sync::atomic::AtomicUsize);
#[async_trait::async_trait]
impl app_application::ports::llm::LlmProvider for Provider {
    async fn stream_chat(
        &self,
        request: app_application::models::chat::ChatRequest,
    ) -> Result<app_application::ports::llm::ChunkStream, app_application::ports::llm::LlmError>
    {
        use app_application::models::chat::{ChatChunk, FinishReason};
        let text = serde_json::to_string(&request).unwrap();
        assert!(!text.contains("HOST_PRIVATE_STORY"));
        assert!(!text.contains("HOST_PRIVATE_SOLO_CHAT"));
        assert!(request.tools.iter().all(|tool| matches!(
            tool.name.as_str(),
            "adventure_action" | "combat_action" | "rest_action"
        )));
        let chunks = if self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            vec![
                ChatChunk::ToolCallStart {
                    id: "model-move".into(),
                    name: "adventure_action".into(),
                },
                ChatChunk::ToolCallArgsDelta {
                    id: "model-move".into(),
                    args_fragment: json!({"action":{"kind":"move","location":"archive"}})
                        .to_string(),
                },
                ChatChunk::ToolCallDone {
                    id: "model-move".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::ToolUse,
                },
            ]
        } else {
            vec![
                ChatChunk::TextDelta {
                    text: "Мы достигли архива.".into(),
                },
                ChatChunk::Done {
                    reason: FinishReason::Stop,
                },
            ]
        };
        Ok(futures::stream::iter(chunks.into_iter().map(Ok)).boxed())
    }
    fn name(&self) -> &'static str {
        "fixture"
    }
    fn active_model(&self) -> &str {
        "fixture"
    }
    fn capabilities_for_model(&self, _: &str) -> app_application::models::chat::Capabilities {
        app_application::models::chat::Capabilities {
            tool_calls: true,
            ..Default::default()
        }
    }
}
fn engine(
    ports: Arc<RoomAgentPorts>,
    provider: Arc<Provider>,
) -> Arc<app_application::agent::turn::AgentTurnService> {
    use app_application::agent::{
        tool_dispatch::NoopRuntimeCoordinator,
        tools::{AgentToolCatalog, ToolAvailability},
        turn::{AgentTurnConfig, AgentTurnService, NeverCancelled},
    };
    let catalog = app_application::rooms::agent::scoped_catalog(AgentToolCatalog {
        exploration: app_server::agent::tools::tools_for_phase(ToolAvailability::default(), false),
        combat: app_server::agent::tools::tools_for_phase(ToolAvailability::default(), true),
    });
    Arc::new(AgentTurnService::new(
        provider,
        ports.clone(),
        ports.clone(),
        ports,
        Arc::new(NoopRuntimeCoordinator),
        Arc::new(app_application::ports::events::NoopApplicationEventSink),
        Arc::new(NeverCancelled),
        AgentTurnConfig {
            model: "fixture".into(),
            system_prompt: "DM".into(),
            temperature: 0.0,
            max_rounds: 4,
            embedding_model: "unused".into(),
            reasoning_enabled: false,
            reasoning_budget: app_application::models::chat::ReasoningSpec::Low,
            tool_availability: ToolAvailability::default(),
            tools: catalog,
        },
    ))
}

#[tokio::test]
async fn room_agent_context_and_tools_use_the_verified_guest_without_private_host_history() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Room agent".into(),
            premise: "Ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let host_sheet = json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[],"backstory":"HOST_PRIVATE_STORY"});
    let current = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: Uuid::new_v4(),
            expected_revision: campaign.revision,
            causation_id: "hero".into(),
            character: host_sheet.clone(),
        })
        .await
        .unwrap();
    store
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::user_text("HOST_PRIVATE_SOLO_CHAT"),
        )
        .await
        .unwrap();
    let rooms = Arc::new(RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(SystemRoomClock),
    ));
    let created = rooms
        .create_local_room(CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "room".into(),
        })
        .await
        .unwrap();
    let guest = rooms
        .join(JoinRoom {
            room_id: created.grant.room.room_id,
            join_id: Uuid::new_v4(),
            display_name: "Guest".into(),
            invitation: RoomSecret::from_input(created.invitation.expose().into()).unwrap(),
            resume_proof: RoomSecret::generate(),
        })
        .await
        .unwrap();
    let guest_subject = rooms
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let host_subject = rooms
        .authenticate(
            created.grant.room.room_id,
            created.grant.member_id,
            &created.grant.capability,
        )
        .await
        .unwrap();
    let character_id = Uuid::new_v4();
    let proposal_id = Uuid::new_v4();
    let mut sheet = host_sheet;
    sheet["backstory"] = json!("GUEST_PRIVATE_STORY");
    sheet["characterId"] = json!(character_id);
    rooms
        .submit_character(
            &guest_subject,
            SubmitRoomCharacter {
                proposal_id,
                character_id,
                character: sheet,
            },
        )
        .await
        .unwrap();
    let party = store.party(campaign.id).await.unwrap().unwrap();
    rooms
        .approve_character(
            &host_subject,
            ApproveRoomCharacter {
                proposal_id,
                expected_revision: party.revision,
                causation_id: "accept".into(),
            },
        )
        .await
        .unwrap();
    let turn_id = Uuid::new_v4();
    let ports = RoomAgentPorts::new(rooms.clone(), guest_subject, turn_id);
    let context = ports
        .build(AgentContextRequest {
            campaign_id: campaign.id.0,
            player_message: "Иду к причалу".into(),
            base_system_prompt: "DM".into(),
            embedding_model: "unused".into(),
        })
        .await
        .unwrap();
    assert!(context.system_prompt.contains("GUEST_PRIVATE_STORY"));
    assert!(!context.system_prompt.contains("HOST_PRIVATE_STORY"));
    assert!(!context.system_prompt.contains("HOST_PRIVATE_SOLO_CHAT"));
    let request = ToolDispatchRequest {
        request_id: turn_id.to_string(),
        tool_call_id: "call-long-id-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .into(),
        campaign_id: campaign.id.0,
        session_id: campaign.last_session_id.0,
        command: AgentToolCommand::AdventureAction(AdventureActionCommand {
            action: app_application::models::adventure::Action::Move(
                app_application::models::adventure::Location::Quay,
            ),
        }),
    };
    let first = ports.execute(request.clone()).await.unwrap();
    assert!(!first.is_error);
    let before = store.party(campaign.id).await.unwrap().unwrap();
    let replay = ports.execute(request.clone()).await.unwrap();
    assert!(!replay.is_error);
    assert!(store.party(campaign.id).await.unwrap().unwrap() == before);
    let forbidden = ports
        .execute(ToolDispatchRequest {
            tool_call_id: "cheat".into(),
            command: AgentToolCommand::ApplyHealing(ApplyHealingCommand {
                token_id: character_id.to_string(),
                amount: 999,
            }),
            ..request.clone()
        })
        .await
        .unwrap();
    assert!(forbidden.is_error);
    let subject = rooms
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let turn = Arc::new(RoomAgentPorts::new(rooms.clone(), subject, Uuid::new_v4()));
    let provider = Arc::new(Provider(std::sync::atomic::AtomicUsize::new(0)));
    let command = turn.command("Иду в архив".into()).await.unwrap();
    let events = engine(turn.clone(), provider.clone())
        .execute(command.clone())
        .collect::<Vec<_>>()
        .await;
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert!(events.iter().any(|event| matches!(
        event,
        Ok(app_application::models::agent::AgentEvent::AgentDone { .. })
    )));
    let checkpoint = turn.checkpoint().await.unwrap();
    assert_eq!(checkpoint.len(), 4);
    assert!(matches!(
        checkpoint.last().unwrap().message,
        app_application::models::chat::ChatMessage::Assistant { .. }
    ));
    let saved_party = store.party(campaign.id).await.unwrap().unwrap();
    let saved_messages = store.list(campaign.last_session_id.0).await.unwrap();
    let replay = engine(turn.clone(), provider.clone())
        .execute_recovered(
            command,
            Some(
                checkpoint
                    .into_iter()
                    .map(|stored| stored.message)
                    .collect(),
            ),
        )
        .collect::<Vec<_>>()
        .await;
    assert!(replay.iter().all(Result::is_ok), "{replay:?}");
    assert_eq!(provider.0.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(
        store.party(campaign.id).await.unwrap().unwrap(),
        saved_party
    );
    assert_eq!(
        store.list(campaign.last_session_id.0).await.unwrap(),
        saved_messages
    );
    assert!(turn.command("Другой запрос".into()).await.is_err());
    let recovering_subject = rooms
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let recovering_id = Uuid::new_v4();
    let recovering = Arc::new(RoomAgentPorts::new(
        rooms.clone(),
        recovering_subject,
        recovering_id,
    ));
    let recovery_command = recovering
        .command("Возвращаюсь к причалу".into())
        .await
        .unwrap();
    recovering
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::user_text("Возвращаюсь к причалу"),
        )
        .await
        .unwrap();
    recovering
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::AssistantWithToolCalls {
                content: None,
                tool_calls: vec![app_application::models::chat::ToolCall {
                    id: "pending-move".into(),
                    name: "adventure_action".into(),
                    args: json!({"action":{"kind":"move","location":"quay"}}),
                }],
            },
        )
        .await
        .unwrap();
    let pending_request = ToolDispatchRequest {
        request_id: recovering_id.to_string(),
        tool_call_id: "pending-move".into(),
        ..request.clone()
    };
    assert!(!recovering.execute(pending_request).await.unwrap().is_error);
    // The process stopped after the effect committed, before its tool result was appended.
    let after_effect = store.party(campaign.id).await.unwrap().unwrap();
    let saved = recovering
        .checkpoint()
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    let recovery_provider = Arc::new(Provider(std::sync::atomic::AtomicUsize::new(1)));
    let resumed = engine(recovering.clone(), recovery_provider)
        .execute_recovered(recovery_command, Some(saved))
        .collect::<Vec<_>>()
        .await;
    assert!(resumed.iter().all(Result::is_ok), "{resumed:?}");
    assert_eq!(
        store.party(campaign.id).await.unwrap().unwrap(),
        after_effect
    );
    assert_eq!(recovering.checkpoint().await.unwrap().len(), 4);
    for action in [
        app_application::models::adventure::Action::Move(
            app_application::models::adventure::Location::Archive,
        ),
        app_application::models::adventure::Action::LocateTikhon,
        app_application::models::adventure::Action::StartDefense,
    ] {
        let current = store.load_current(campaign.id).await.unwrap();
        rooms
            .adventure(
                &host_subject,
                app_application::models::room_world::RoomAdventureCommand {
                    request_id: Uuid::new_v4().to_string(),
                    expected_revision: current.campaign.revision,
                    action,
                },
            )
            .await
            .unwrap();
    }
    use app_application::ports::repositories::CombatRepository;
    let current = store.load_current(campaign.id).await.unwrap();
    let mut fight: app_application::models::combat::CombatProjection =
        serde_json::from_value(current.projection.combat.unwrap()).unwrap();
    let old_revision = fight.revision;
    let guest_actor = fight
        .snapshot
        .combatants
        .iter_mut()
        .find(|actor| actor.character_id.is_some_and(|id| id.0 == character_id))
        .unwrap();
    guest_actor.position = app_domain::combat::types::Position { x: 1, y: 3 };
    fight.snapshot.current_combatant = Some(guest_actor.id);
    fight.revision += 1;
    store.compare_and_set(old_revision, fight).await.unwrap();
    let combat_subject = rooms
        .authenticate(guest.room.room_id, guest.member_id, &guest.capability)
        .await
        .unwrap();
    let combat_id = Uuid::new_v4();
    let combat_ports = Arc::new(RoomAgentPorts::new(
        rooms.clone(),
        combat_subject,
        combat_id,
    ));
    let combat_command = combat_ports.command("Отступаю".into()).await.unwrap();
    combat_ports
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::user_text("Отступаю"),
        )
        .await
        .unwrap();
    combat_ports
        .append(
            campaign.last_session_id.0,
            app_application::models::chat::ChatMessage::AssistantWithToolCalls {
                content: None,
                tool_calls: vec![app_application::models::chat::ToolCall {
                    id: "terminal-retreat".into(),
                    name: "combat_action".into(),
                    args: json!({"kind":"retreat"}),
                }],
            },
        )
        .await
        .unwrap();
    let terminal = combat_ports
        .execute(ToolDispatchRequest {
            request_id: combat_id.to_string(),
            tool_call_id: "terminal-retreat".into(),
            command: AgentToolCommand::CombatAction(
                app_application::agent::hero_intentions::HeroCombatIntent::Retreat,
            ),
            ..request.clone()
        })
        .await
        .unwrap();
    assert!(!terminal.is_error, "{terminal:?}");
    assert!(store
        .load_current(campaign.id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
    let saved = combat_ports
        .checkpoint()
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    let resumed = engine(
        combat_ports.clone(),
        Arc::new(Provider(std::sync::atomic::AtomicUsize::new(1))),
    )
    .execute_recovered(combat_command, Some(saved))
    .collect::<Vec<_>>()
    .await;
    assert!(resumed.iter().all(Result::is_ok), "{resumed:?}");
    let checkpoint = combat_ports.checkpoint().await.unwrap();
    match &checkpoint[2].message {
        app_application::models::chat::ChatMessage::ToolResult(result) => {
            assert!(!result.is_error, "{result:?}")
        }
        other => panic!("Expected recovered receipt, got {other:?}"),
    };
    let current = store.load_current(campaign.id).await.unwrap();
    let snapshot = app_application::campaigns::CreateSnapshot::new(store.clone())
        .execute(app_application::campaigns::CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            title: "Agent checkpoint".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "".into(),
            causation_id: "agent-save".into(),
            character: current.projection.character,
        })
        .await
        .unwrap();
    let current = store.load_current(campaign.id).await.unwrap();
    store
        .restore_snapshot(app_application::models::snapshot::RestoreSnapshotRecord {
            save_id: snapshot.save_id,
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "agent-restore".into(),
        })
        .await
        .unwrap();
    assert!(turn.checkpoint().await.is_err());
    assert!(turn.command("Иду в архив".into()).await.is_err());
    assert!(ports.execute(request.clone()).await.is_err());
    rooms.revoke(&host_subject, guest.member_id).await.unwrap();
    assert!(ports
        .execute(ToolDispatchRequest {
            tool_call_id: "revoked".into(),
            ..request
        })
        .await
        .is_err());
}
