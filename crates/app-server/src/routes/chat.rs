//! Compatibility exports for the Axum chat adapter.

pub use adapter_http::routes::chat::{chat, ChatHttpRequest, HttpMessage};

use app_llm::{ChatMessage, MessagePart};

use crate::error::AppError;

const MAX_IMAGES_PER_MESSAGE: usize = 4;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Temporary compatibility helper for the legacy focused tests.
pub fn enforce_size_guards(messages: &[ChatMessage]) -> Result<(), AppError> {
    use base64::Engine;
    for message in messages {
        if let ChatMessage::User { parts } = message {
            let count = parts
                .iter()
                .filter(|part| matches!(part, MessagePart::Image { .. }))
                .count();
            if count > MAX_IMAGES_PER_MESSAGE {
                return Err(AppError::PayloadTooLarge(format!(
                    "at most {MAX_IMAGES_PER_MESSAGE} image parts per message (got {count})"
                )));
            }
            for part in parts {
                if let MessagePart::Image { data_b64, .. } = part {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(data_b64)
                        .map_err(|error| {
                            AppError::BadRequest(format!("invalid base64 image: {error}"))
                        })?;
                    if bytes.len() > MAX_IMAGE_BYTES {
                        return Err(AppError::PayloadTooLarge(format!(
                            "image exceeds 5 MB (got {} bytes)",
                            bytes.len()
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}
