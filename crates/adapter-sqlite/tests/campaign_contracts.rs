use std::collections::BTreeMap;

use adapter_sqlite::{pool, SqliteStore};
use app_application::models::identity::{CampaignId, Revision, SessionId};
use app_application::models::session::{
    CampaignDetail, CampaignEvent, CampaignSeed, CampaignSession, InitialCampaign,
};
use app_application::ports::repositories::{
    CampaignRepository, EventRepository, RepositoryError, SessionRepository,
};
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

async fn store() -> SqliteStore {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&pool).await.unwrap();
    SqliteStore::new(pool)
}

fn initial_campaign(
    campaign_id: CampaignId,
    session_id: SessionId,
    event_id: Uuid,
) -> InitialCampaign {
    let seed = CampaignSeed::Collaborative {
        answers: BTreeMap::from([("genre".into(), vec!["classic_fantasy".into()])]),
    };
    let detail = CampaignDetail {
        id: campaign_id,
        schema_version: 1,
        ruleset_id: "srd-5.2.1".into(),
        title: "Серебряная долина".into(),
        premise: "Древняя дорога проснулась".into(),
        seed,
        last_session_id: session_id,
        revision: Revision(1),
        created_at_epoch_ms: 1_700_000_000_000,
        updated_at_epoch_ms: 1_700_000_000_000,
    };
    InitialCampaign {
        opening_scene: None,
        session: CampaignSession {
            id: session_id,
            campaign_id,
            revision: Revision(1),
            created_at_epoch_ms: 1_700_000_000_000,
            updated_at_epoch_ms: 1_700_000_000_000,
        },
        event: CampaignEvent {
            event_id,
            campaign_id,
            revision: Revision(1),
            kind: "campaign_created".into(),
            payload: json!({
                "title": "Серебряная долина",
                "seed": detail.seed,
            }),
            causation_id: Some("create-1".into()),
            created_at_epoch_ms: 1_700_000_000_000,
        },
        detail,
    }
}

fn event(campaign_id: CampaignId, revision: u64, kind: &str) -> CampaignEvent {
    CampaignEvent {
        event_id: Uuid::new_v4(),
        campaign_id,
        revision: Revision(revision),
        kind: kind.into(),
        payload: json!({"kind": kind}),
        causation_id: None,
        created_at_epoch_ms: 1_700_000_000_000 + i64::try_from(revision).unwrap(),
    }
}

#[tokio::test]
async fn campaign_create_round_trips_and_rolls_back_all_rows_on_event_failure() {
    let store = store().await;
    let campaign_id = CampaignId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    let shared_event_id = Uuid::new_v4();
    let created = CampaignRepository::create(
        &store,
        initial_campaign(campaign_id, session_id, shared_event_id),
    )
    .await
    .unwrap();

    assert_eq!(
        CampaignRepository::get(&store, campaign_id).await.unwrap(),
        Some(created.clone())
    );
    assert_eq!(CampaignRepository::list(&store).await.unwrap().len(), 1);
    assert_eq!(
        SessionRepository::get(&store, session_id).await.unwrap(),
        Some(CampaignSession {
            id: session_id,
            campaign_id,
            revision: Revision(1),
            created_at_epoch_ms: 1_700_000_000_000,
            updated_at_epoch_ms: 1_700_000_000_000,
        })
    );

    let rejected_campaign = CampaignId(Uuid::new_v4());
    let rejected_session = SessionId(Uuid::new_v4());
    let rejected = CampaignRepository::create(
        &store,
        initial_campaign(rejected_campaign, rejected_session, shared_event_id),
    )
    .await;
    assert!(rejected.is_err());
    assert!(CampaignRepository::get(&store, rejected_campaign)
        .await
        .unwrap()
        .is_none());
    assert!(SessionRepository::get(&store, rejected_session)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn event_append_is_monotonic_and_rejects_a_stale_revision_without_writes() {
    let store = store().await;
    let campaign_id = CampaignId(Uuid::new_v4());
    CampaignRepository::create(
        &store,
        initial_campaign(campaign_id, SessionId(Uuid::new_v4()), Uuid::new_v4()),
    )
    .await
    .unwrap();

    let rev2 = EventRepository::append(
        &store,
        campaign_id,
        Revision(1),
        vec![event(campaign_id, 2, "scene_prepared")],
    )
    .await
    .unwrap();
    assert_eq!(rev2, Revision(2));

    let stale = EventRepository::append(
        &store,
        campaign_id,
        Revision(1),
        vec![event(campaign_id, 2, "stale")],
    )
    .await;
    assert!(matches!(
        stale,
        Err(RepositoryError::Conflict {
            expected: Revision(1),
            actual: Revision(2),
        })
    ));

    let events = EventRepository::since(&store, campaign_id, Revision(1))
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind.as_str(), "scene_prepared");
    assert_eq!(events[0].revision, Revision(2));
}
