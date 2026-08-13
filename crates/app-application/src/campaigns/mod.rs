mod create;
mod get;
mod list;
mod snapshots;

pub use app_domain::ruleset::{GAME_SCHEMA_VERSION, RULESET_ID};
pub use create::{CreateCampaign, CreateCampaignCommand};
pub use get::GetCampaign;
pub use list::ListCampaigns;
pub use snapshots::{
    CreateSnapshot, CreateSnapshotCommand, RestoreSnapshot, RestoreSnapshotCommand,
};

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CampaignError {
    #[error("campaign input rejected with code {code}")]
    InvalidInput { code: &'static str },
    #[error("campaign persistence failed with code {code}")]
    Persistence { code: &'static str },
}
