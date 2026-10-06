use app_application::{
    combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    },
    models::combat::{CombatActionCommit, CombatProjection},
    ports::repositories::{CombatRepository, RepositoryError},
};
use app_domain::combat::types::CombatantId;
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;
struct EndedAfterReceiptRead {
    reads: AtomicUsize,
    projection: CombatProjection,
}
#[async_trait]
impl CombatRepository for EndedAfterReceiptRead {
    async fn find_action(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
    ) -> Result<Option<CombatProjection>, RepositoryError> {
        Ok((self.reads.fetch_add(1, Ordering::SeqCst) > 0).then(|| self.projection.clone()))
    }
    async fn get(&self, _: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        Ok(None)
    }
    async fn create(
        &self,
        _: Uuid,
        _: CombatProjection,
    ) -> Result<CombatProjection, RepositoryError> {
        unreachable!()
    }
    async fn commit_action(
        &self,
        _: u64,
        _: CombatProjection,
        _: &str,
        _: &str,
    ) -> Result<CombatActionCommit, RepositoryError> {
        unreachable!()
    }
    async fn compare_and_set(&self, _: u64, _: CombatProjection) -> Result<(), RepositoryError> {
        unreachable!()
    }
    async fn end(&self, _: Uuid) -> Result<Option<CombatProjection>, RepositoryError> {
        unreachable!()
    }
}
#[tokio::test]
async fn final_duplicate_rechecks_the_receipt_when_the_active_encounter_disappears() {
    let id = Uuid::new_v4();
    let repository = Arc::new(EndedAfterReceiptRead {
        reads: AtomicUsize::new(0),
        projection: CombatProjection::empty(id, 1),
    });
    let result = ResolveCombatAction::new(repository)
        .execute(ResolveCombatCommand {
            request_id: "final".into(),
            encounter_id: id,
            expected_revision: 0,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn {
                combatant: CombatantId::new(),
            },
        })
        .await
        .unwrap();
    assert!(result.duplicate);
    assert_eq!(result.projection.revision, 1);
}
