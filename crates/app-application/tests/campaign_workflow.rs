use std::sync::Arc;

use app_application::campaigns::{CreateCampaign, CreateCampaignCommand, ListCampaigns};
use app_application::models::identity::Revision;
use app_application::models::session::{
    CampaignDetail, CampaignSeed, CampaignSummary, InitialCampaign,
};
use app_application::ports::repositories::{CampaignRepository, RepositoryError};
use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Default)]
struct InMemoryCampaigns {
    records: RwLock<Vec<InitialCampaign>>,
}

#[async_trait]
impl CampaignRepository for InMemoryCampaigns {
    async fn create(&self, campaign: InitialCampaign) -> Result<CampaignDetail, RepositoryError> {
        let detail = campaign.detail.clone();
        self.records.write().await.push(campaign);
        Ok(detail)
    }

    async fn list(&self) -> Result<Vec<CampaignSummary>, RepositoryError> {
        Ok(self
            .records
            .read()
            .await
            .iter()
            .map(|record| CampaignSummary::from(&record.detail))
            .collect())
    }

    async fn get(
        &self,
        campaign_id: app_application::models::identity::CampaignId,
    ) -> Result<Option<CampaignDetail>, RepositoryError> {
        Ok(self
            .records
            .read()
            .await
            .iter()
            .find(|record| record.detail.id == campaign_id)
            .map(|record| record.detail.clone()))
    }
}

fn answers() -> std::collections::BTreeMap<String, Vec<String>> {
    std::collections::BTreeMap::from([
        ("genre".into(), vec!["classic_fantasy".into()]),
        ("tone".into(), vec!["hopeful".into(), "mysterious".into()]),
    ])
}

#[tokio::test]
async fn creation_commits_one_srd_521_aggregate_and_list_returns_it() {
    let repository = Arc::new(InMemoryCampaigns::default());
    let create = CreateCampaign::new(repository.clone());
    let list = ListCampaigns::new(repository.clone());

    let detail = create
        .execute(CreateCampaignCommand {
            title: "Серебряная долина".into(),
            premise: "Древняя дорога проснулась".into(),
            seed: CampaignSeed::Collaborative { answers: answers() },
        })
        .await
        .unwrap();

    assert_eq!(detail.ruleset_id, "srd-5.2.1");
    assert_eq!(detail.schema_version, 1);
    assert_eq!(detail.revision, Revision(1));
    let listed = list.execute().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, detail.id);
    assert_eq!(listed[0].last_session_id, detail.last_session_id);

    let records = repository.records.read().await;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].session.campaign_id, detail.id);
    assert_eq!(records[0].session.id, detail.last_session_id);
    assert_eq!(records[0].event.campaign_id, detail.id);
    assert_eq!(records[0].event.revision, Revision(1));
    assert_eq!(records[0].event.kind.as_str(), "campaign_created");
}
