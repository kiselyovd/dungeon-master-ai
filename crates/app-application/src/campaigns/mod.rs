mod create;
mod list;

pub use create::{CreateCampaign, CreateCampaignCommand};
pub use list::ListCampaigns;

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CampaignError {
    #[error("campaign input rejected with code {code}")]
    InvalidInput { code: &'static str },
    #[error("campaign persistence failed with code {code}")]
    Persistence { code: &'static str },
}
