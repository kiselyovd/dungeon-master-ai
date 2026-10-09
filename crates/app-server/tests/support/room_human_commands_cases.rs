use super::*;

#[tokio::test]
async fn room_clarification_is_private_idempotent_and_cannot_rewrite_resolved_goals() {
    let f = fixture().await;
    let (ledger, _, mut manifest) = journal(&f).await;
    manifest.commands[0].goal = HumanCommandGoal::NeedsResolution {
        description: "Длительность ожидания".into(),
    };
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let question = ledger
        .record_clarification(1, "Сколько минут ждать?".into())
        .await
        .unwrap();
    assert_eq!(
        ledger
            .record_clarification(1, question.question.clone())
            .await
            .unwrap(),
        question
    );
    assert!(ledger
        .prepare_after_permission(proposal(1, "too-early"))
        .await
        .is_err());
    assert!(ledger
        .resolve_verified_goal(1, HumanAction::Rest(RestIntent::Wait { minutes: 1 }))
        .await
        .is_err());
    let guest = f
        .rooms
        .authenticate(f.guest.room.room_id, f.guest.member_id, &f.guest.capability)
        .await
        .unwrap();
    let foreign = RoomHumanCommandLedger::new(
        f.store.as_ref().clone(),
        guest.lease().clone(),
        manifest.scope.request_id,
        Uuid::new_v4(),
    );
    assert!(foreign
        .submit_clarification_reply(question.id, "60".into())
        .await
        .is_err());
    ledger
        .submit_clarification_reply(question.id, "Одну минуту".into())
        .await
        .unwrap();
    ledger
        .submit_clarification_reply(question.id, "Одну минуту".into())
        .await
        .unwrap();
    assert!(ledger
        .submit_clarification_reply(question.id, "Десять".into())
        .await
        .is_err());
    ledger
        .resolve_verified_goal(1, HumanAction::Rest(RestIntent::Wait { minutes: 1 }))
        .await
        .unwrap();
    assert!(ledger
        .resolve_verified_goal(1, HumanAction::Rest(RestIntent::Wait { minutes: 10 }))
        .await
        .is_err());
    let saved = ledger.load_current().await.unwrap().unwrap();
    assert_eq!(saved.manifest.scope, manifest.scope);
    assert_eq!(
        saved.manifest.commands[0].source,
        manifest.commands[0].source
    );
    assert_eq!(saved.manifest.commands[1], manifest.commands[1]);
    assert_eq!(saved.clarifications.len(), 1);
    ledger
        .prepare_after_permission(proposal(1, "accepted"))
        .await
        .unwrap();
}
#[path = "room_command_runtime_cases.rs"]
mod runtime;
use adapter_sqlite::room_human_commands::RoomHumanCommandLedger;
use app_application::{
    agent::{
        human_commands::*,
        tool_dispatch::{ToolDispatchRequest, ToolDispatcher},
    },
    models::rest::RestIntent,
};

async fn journal(
    f: &Fixture,
) -> (
    RoomHumanCommandLedger,
    Arc<RoomAgentPorts>,
    HumanCommandManifest,
) {
    let turn = Uuid::new_v4();
    let ports = f.ports(&f.host, turn).await;
    let text = "Жду минуту и ещё жду минуту";
    let input = operation(&ports, text).await;
    let owner = Uuid::new_v4();
    ports.claim(input, owner).await.unwrap();
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let lease = subject.lease().clone();
    let party = f.store.party(f.campaign).await.unwrap().unwrap();
    let actor = party
        .roster
        .members
        .iter()
        .find(|m| m.member_id == f.host.member_id)
        .unwrap()
        .character_id;
    let reset: i64 = sqlx::query_scalar("SELECT reset_cursor FROM room_dm_turns WHERE turn_id=?1")
        .bind(turn.to_string())
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    let manifest = HumanCommandManifest {
        version: HUMAN_COMMAND_VERSION,
        scope: CommandScope {
            request_id: turn,
            campaign_id: f.campaign.0,
            session_id: f.session.0,
            actor_id: actor,
            authority: CommandAuthority::Room {
                room_id: lease.room_id(),
                member_id: lease.member_id(),
                epoch: lease.epoch(),
                reset_cursor: reset as u64,
            },
        },
        commands: vec![
            HumanCommand {
                id: 1,
                source: CommandSpan::from_quote(text, "Жду минуту", 0).unwrap(),
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Rest(RestIntent::Wait { minutes: 1 }),
                },
                condition: None,
            },
            HumanCommand {
                id: 2,
                source: CommandSpan::from_quote(text, "жду минуту", 0).unwrap(),
                goal: HumanCommandGoal::Exact {
                    action: HumanAction::Rest(RestIntent::Wait { minutes: 1 }),
                },
                condition: None,
            },
        ],
        constraints: vec![],
    };
    (
        RoomHumanCommandLedger::new(f.store.as_ref().clone(), lease, turn, owner),
        ports,
        manifest,
    )
}
fn proposal(id: u16, call: &str) -> HumanCommandStepProposal {
    HumanCommandStepProposal {
        command_id: id,
        tool_call_id: call.into(),
        signature: HumanAction::Rest(RestIntent::Wait { minutes: 1 }),
        terminal: true,
    }
}

#[tokio::test]
async fn legacy_prepare_cannot_bypass_a_turn_with_a_saved_manifest() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let before = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let result = ports
        .execute(ToolDispatchRequest {
            request_id: manifest.scope.request_id.to_string(),
            tool_call_id: "unbound".into(),
            campaign_id: f.campaign.0,
            session_id: f.session.0,
            command: proposal(1, "unbound").signature.to_tool(),
        })
        .await;
    assert!(result.is_err() || result.unwrap().is_error);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        before
    );
    assert!(ledger
        .prepare_after_permission(proposal(1, "unbound"))
        .await
        .is_err());
}

#[tokio::test]
async fn room_journal_replays_effect_without_transcript_and_preserves_two_identical_goals() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let first = ledger
        .prepare_after_permission(proposal(1, "first"))
        .await
        .unwrap();
    assert_eq!(
        ledger
            .prepare_after_permission(proposal(1, "first"))
            .await
            .unwrap(),
        first
    );
    assert!(ledger
        .prepare_after_permission(proposal(2, "early"))
        .await
        .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM room_agent_tools WHERE tool_id='early'")
            .fetch_one(f.store.pool())
            .await
            .unwrap();
    assert_eq!(count, 0, "failed binding must roll back real preparation");
    let request = ToolDispatchRequest {
        request_id: manifest.scope.request_id.to_string(),
        tool_call_id: "first".into(),
        campaign_id: f.campaign.0,
        session_id: f.session.0,
        command: first.signature.to_tool(),
    };
    assert!(!ports.execute(request.clone()).await.unwrap().is_error);
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let recovered = ledger.load_current().await.unwrap().unwrap();
    assert_eq!(recovered.receipts.len(), 1);
    assert_eq!(
        recovered
            .manifest
            .progress(&recovered.bindings, &recovered.receipts)
            .unwrap()[1]
            .1,
        HumanCommandProgress::Ready
    );
    assert!(!ports.execute(request).await.unwrap().is_error);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        revision
    );
    let second = ledger
        .prepare_after_permission(proposal(2, "second"))
        .await
        .unwrap();
    assert_ne!(first.canonical_nonce, second.canonical_nonce);
    assert!(f
        .store
        .room_agent_checkpoint(
            &f.rooms
                .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
                .await
                .unwrap()
                .lease()
                .clone(),
            manifest.scope.request_id
        )
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn room_journal_rejects_changed_source_scope_and_foreign_owner() {
    let f = fixture().await;
    let (ledger, _, manifest) = journal(&f).await;
    let mut wrong = manifest.clone();
    wrong.commands[0].source.quote = "Отдыхаю".into();
    assert!(ledger.save_verified_manifest(wrong).await.is_err());
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let mut changed = manifest.clone();
    changed.commands.pop();
    assert!(ledger.save_verified_manifest(changed).await.is_err());
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let foreign = RoomHumanCommandLedger::new(
        f.store.as_ref().clone(),
        subject.lease().clone(),
        manifest.scope.request_id,
        Uuid::new_v4(),
    );
    assert!(foreign.load_current().await.is_err());
    let guest = f
        .rooms
        .authenticate(f.guest.room.room_id, f.guest.member_id, &f.guest.capability)
        .await
        .unwrap();
    let foreign = RoomHumanCommandLedger::new(
        f.store.as_ref().clone(),
        guest.lease().clone(),
        manifest.scope.request_id,
        Uuid::new_v4(),
    );
    assert!(foreign.load_current().await.is_err());
}

#[tokio::test]
async fn room_journal_refusal_is_durable_and_cannot_be_executed_afterward() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let binding = ledger
        .prepare_after_permission(proposal(1, "refused"))
        .await
        .unwrap();
    assert!(ledger
        .record_definitive_refusal("refused", "network_timeout")
        .await
        .is_err());
    ledger
        .record_definitive_refusal("refused", "rest_unsafe")
        .await
        .unwrap();
    ledger
        .record_definitive_refusal("refused", "rest_unsafe")
        .await
        .unwrap();
    assert!(ledger
        .record_definitive_refusal("refused", "rest_too_soon")
        .await
        .is_err());
    let before = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let result = ports
        .execute(ToolDispatchRequest {
            request_id: manifest.scope.request_id.to_string(),
            tool_call_id: "refused".into(),
            campaign_id: f.campaign.0,
            session_id: f.session.0,
            command: binding.signature.to_tool(),
        })
        .await;
    assert!(result.is_err() || result.unwrap().is_error);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        before
    );
    let recovered = ledger.load_current().await.unwrap().unwrap();
    assert_eq!(
        recovered.receipts[0].outcome,
        CommandAttemptOutcome::Refused {
            code: "rest_unsafe".into()
        }
    );
    ledger
        .prepare_after_permission(proposal(2, "next"))
        .await
        .unwrap();
}

#[tokio::test]
async fn room_journal_pending_revision_change_blocks_both_load_and_effect() {
    use app_application::models::{identity::Revision, room_world::RoomRestCommand};
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let binding = ledger
        .prepare_after_permission(proposal(1, "pending"))
        .await
        .unwrap();
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    f.rooms
        .rest(
            &subject,
            RoomRestCommand {
                request_id: Uuid::new_v4().to_string(),
                expected_revision: Revision(revision.0),
                intent: RestIntent::Wait { minutes: 2 },
            },
        )
        .await
        .unwrap();
    assert!(ledger.load_current().await.is_err());
    assert!(ledger
        .record_definitive_refusal("pending", "rest_unsafe")
        .await
        .is_err());
    let before = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let result = ports
        .execute(ToolDispatchRequest {
            request_id: manifest.scope.request_id.to_string(),
            tool_call_id: "pending".into(),
            campaign_id: f.campaign.0,
            session_id: f.session.0,
            command: binding.signature.to_tool(),
        })
        .await;
    assert!(result.is_err() || result.unwrap().is_error);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        before
    );
}

#[tokio::test]
async fn room_journal_exclusive_recovery_transfers_owner_without_repeating_effect() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let binding = ledger
        .prepare_after_permission(proposal(1, "committed"))
        .await
        .unwrap();
    assert!(
        !ports
            .execute(ToolDispatchRequest {
                request_id: manifest.scope.request_id.to_string(),
                tool_call_id: "committed".into(),
                campaign_id: f.campaign.0,
                session_id: f.session.0,
                command: binding.signature.to_tool()
            })
            .await
            .unwrap()
            .is_error
    );
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let next_owner = Uuid::new_v4();
    assert_eq!(
        f.store
            .recover_after_exclusive_authority(next_owner)
            .await
            .unwrap(),
        1
    );
    assert!(ledger.load_current().await.is_err());
    assert!(matches!(
        ports
            .resume(
                operation(&ports, "Жду минуту и ещё жду минуту").await,
                next_owner
            )
            .await
            .unwrap(),
        OperationClaim::Resuming { .. }
    ));
    let subject = f
        .rooms
        .authenticate(f.host.room.room_id, f.host.member_id, &f.host.capability)
        .await
        .unwrap();
    let recovered = RoomHumanCommandLedger::new(
        f.store.as_ref().clone(),
        subject.lease().clone(),
        manifest.scope.request_id,
        next_owner,
    )
    .load_current()
    .await
    .unwrap()
    .unwrap();
    assert_eq!(recovered.receipts.len(), 1);
    assert_eq!(recovered.bindings[0], binding);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        revision
    );
}

#[tokio::test]
async fn room_journal_reset_cursor_invalidates_saved_manifest_and_prepared_action() {
    let f = fixture().await;
    let (ledger, ports, manifest) = journal(&f).await;
    ledger
        .save_verified_manifest(manifest.clone())
        .await
        .unwrap();
    let binding = ledger
        .prepare_after_permission(proposal(1, "old-timeline"))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO room_changes(room_id,kind,payload_json) VALUES(?1,'timeline_reset','{}')",
    )
    .bind(f.host.room.room_id.to_string())
    .execute(f.store.pool())
    .await
    .unwrap();
    assert!(ledger.load_current().await.is_err());
    assert!(ledger
        .prepare_after_permission(proposal(1, "old-timeline"))
        .await
        .is_err());
    let revision = f
        .store
        .load_current(f.campaign)
        .await
        .unwrap()
        .campaign
        .revision;
    let result = ports
        .execute(ToolDispatchRequest {
            request_id: manifest.scope.request_id.to_string(),
            tool_call_id: "old-timeline".into(),
            campaign_id: f.campaign.0,
            session_id: f.session.0,
            command: binding.signature.to_tool(),
        })
        .await;
    assert!(result.is_err() || result.unwrap().is_error);
    assert_eq!(
        f.store
            .load_current(f.campaign)
            .await
            .unwrap()
            .campaign
            .revision,
        revision
    );
}
