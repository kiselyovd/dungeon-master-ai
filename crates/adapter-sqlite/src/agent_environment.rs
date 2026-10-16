use app_application::{
    agent::operations::ExecutionEnvironment, ports::repositories::RepositoryError,
};
use sqlx::{sqlite::SqliteRow, Row};

pub(crate) fn validate(
    row: &SqliteRow,
    current: &ExecutionEnvironment,
) -> Result<(), RepositoryError> {
    let provider: Option<String> = row.get("execution_provider");
    let model: Option<String> = row.get("execution_model");
    let code = if provider.is_none() || model.is_none() || current.provider.is_none() {
        Some("agent_original_environment_unavailable")
    } else if provider != current.provider || model.as_deref() != Some(current.model.as_str()) {
        Some("agent_original_environment_changed")
    } else {
        None
    };
    match code {
        Some(code) => Err(RepositoryError::Operation {
            operation: "agent_environment",
            code,
        }),
        None => Ok(()),
    }
}
