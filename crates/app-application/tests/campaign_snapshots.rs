use std::sync::Arc;

use app_application::campaigns::{
    CreateSnapshot, CreateSnapshotCommand, RestoreSnapshot, RestoreSnapshotCommand,
};
use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::snapshot::{
    CampaignSnapshot, CreateSnapshotRecord, RestoreSnapshotRecord, SnapshotCreated,
    SnapshotProjection, SnapshotSaveDetails, SnapshotSaveSummary, SNAPSHOT_SCHEMA_VERSION,
};
use app_application::ports::repositories::{RepositoryError, SnapshotRepository};
use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
struct StubRepository {
    snapshot: CampaignSnapshot,
}

#[async_trait]
impl SnapshotRepository for StubRepository {
    async fn create_snapshot(
        &self,
        _record: CreateSnapshotRecord,
    ) -> Result<SnapshotCreated, RepositoryError> {
        Ok(SnapshotCreated {
            save_id: self.snapshot.id,
            snapshot: self.snapshot.clone(),
        })
    }

    async fn restore_snapshot(
        &self,
        _record: RestoreSnapshotRecord,
    ) -> Result<CampaignSnapshot, RepositoryError> {
        Ok(self.snapshot.clone())
    }

    async fn list_snapshots(
        &self,
        _session_id: SessionId,
    ) -> Result<Vec<SnapshotSaveSummary>, RepositoryError> {
        Ok(vec![])
    }

    async fn get_snapshot(
        &self,
        _save_id: Uuid,
    ) -> Result<Option<SnapshotSaveDetails>, RepositoryError> {
        Ok(None)
    }

    async fn delete_snapshot(&self, _save_id: Uuid) -> Result<bool, RepositoryError> {
        Ok(false)
    }

    async fn update_snapshot_metadata(
        &self,
        _save_id: Uuid,
        _kind: String,
        _title: String,
        _summary: String,
        _tag: String,
    ) -> Result<bool, RepositoryError> {
        Ok(false)
    }
}

fn snapshot(campaign_id: CampaignId, session_id: SessionId) -> CampaignSnapshot {
    CampaignSnapshot {
        id: Uuid::new_v4(),
        campaign_id,
        session_id,
        revision: Revision(4),
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        ruleset_id: "srd-5.2.1".into(),
        projection: SnapshotProjection {
            messages: vec![],
            character: json!({"name": "Mira"}),
            combat: None,
            scene: None,
            journal: vec![],
            npcs: vec![],
        },
    }
}

#[tokio::test]
async fn create_rejects_partial_character_projection_before_persistence() {
    let campaign_id = CampaignId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let use_case = CreateSnapshot::new(Arc::new(StubRepository {
        snapshot: snapshot(campaign_id, session_id),
    }));
    let error = use_case
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id,
            session_id,
            expected_revision: Revision(4),
            title: "Before the gate".into(),
            kind: "manual".into(),
            summary: String::new(),
            tag: "exploration".into(),
            causation_id: "save-1".into(),
            character: serde_json::Value::Null,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("snapshot_invalid"));
}

#[tokio::test]
async fn restore_rejects_cross_campaign_and_cross_session_snapshots() {
    let campaign_id = CampaignId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let use_case = RestoreSnapshot::new(Arc::new(StubRepository {
        snapshot: snapshot(CampaignId(Uuid::new_v4()), SessionId(Uuid::new_v4())),
    }));
    let error = use_case
        .execute(RestoreSnapshotCommand {
            save_id: Uuid::new_v4(),
            campaign_id,
            session_id,
            expected_revision: Revision(4),
            causation_id: "restore-1".into(),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("snapshot_incompatible"));
}

#[tokio::test]
async fn restore_rejects_future_schema_and_wrong_ruleset() {
    let campaign_id = CampaignId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let mut incompatible = snapshot(campaign_id, session_id);
    incompatible.schema_version = SNAPSHOT_SCHEMA_VERSION + 1;
    incompatible.ruleset_id = "srd-future".into();
    let use_case = RestoreSnapshot::new(Arc::new(StubRepository {
        snapshot: incompatible,
    }));
    let error = use_case
        .execute(RestoreSnapshotCommand {
            save_id: Uuid::new_v4(),
            campaign_id,
            session_id,
            expected_revision: Revision(4),
            causation_id: "restore-2".into(),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("snapshot_incompatible"));
}

#[tokio::test]
async fn restore_rejects_partial_projection_before_it_can_reach_an_adapter() {
    let campaign_id = CampaignId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let mut partial = snapshot(campaign_id, session_id);
    partial.projection.scene = Some(json!({ "title": "Missing identity and mode" }));
    let use_case = RestoreSnapshot::new(Arc::new(StubRepository { snapshot: partial }));
    let error = use_case
        .execute(RestoreSnapshotCommand {
            save_id: Uuid::new_v4(),
            campaign_id,
            session_id,
            expected_revision: Revision(4),
            causation_id: "restore-partial".into(),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("snapshot_incompatible"));
}
