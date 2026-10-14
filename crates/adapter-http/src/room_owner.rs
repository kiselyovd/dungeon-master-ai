//! Privileged native room control. Never include this router in public room transport.
use crate::services::{HttpServiceError, NativeRoomHostHttpService};
use app_application::models::room_host::{NativeHostRoomGrant, NativeHostRoomRequest};
use axum::{
    extract::{DefaultBodyLimit, Path},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    routing::{delete, post},
    Extension, Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;
/// Token comes from native process composition; it is neither exposed nor logged.
pub fn router(
    service: Arc<dyn NativeRoomHostHttpService>,
    token: String,
) -> Result<Router, HttpServiceError> {
    protect(
        Router::new()
            .route("/native/rooms/host", post(start))
            .route("/native/rooms/recover", post(recover))
            .route("/native/rooms/{room_id}", delete(stop))
            .layer(DefaultBodyLimit::max(4096))
            .layer(Extension(service)),
        token,
    )
}
pub(crate) fn protect(base: Router, token: String) -> Result<Router, HttpServiceError> {
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HttpServiceError::Internal {
            code: "native_owner_token_invalid",
        });
    }
    Ok(base.layer(middleware::from_fn(
        move |request: axum::extract::Request, next: Next| {
            let expected = token.clone();
            async move {
                let mut values = request.headers().get_all(header::AUTHORIZATION).iter();
                let value = values
                    .next()
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.strip_prefix("Bearer "));
                let allowed = values.next().is_none()
                    && value.is_some_and(|value| equal(value.as_bytes(), expected.as_bytes()));
                let mut response = if allowed {
                    next.run(request).await
                } else {
                    axum::response::IntoResponse::into_response(HttpServiceError::Unauthorized {
                        code: "native_owner_denied",
                    })
                };
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            }
        },
    )))
}
fn equal(actual: &[u8], expected: &[u8]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    actual
        .iter()
        .zip(expected)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}
async fn start(
    Extension(service): Extension<Arc<dyn NativeRoomHostHttpService>>,
    Json(request): Json<NativeHostRoomRequest>,
) -> Result<Json<NativeHostRoomGrant>, HttpServiceError> {
    if request.request_id.is_nil()
        || request.campaign_id.0.is_nil()
        || request.session_id.0.is_nil()
        || request.bind_addr.ip().is_multicast()
    {
        return Err(HttpServiceError::BadRequest {
            code: "native_host_request_invalid",
        });
    }
    service.start(request).await.map(Json)
}
async fn recover(
    Extension(service): Extension<Arc<dyn NativeRoomHostHttpService>>,
) -> Result<Json<Option<NativeHostRoomGrant>>, HttpServiceError> {
    service.recover().await.map(Json)
}
async fn stop(
    Extension(service): Extension<Arc<dyn NativeRoomHostHttpService>>,
    Path(room): Path<Uuid>,
) -> Result<StatusCode, HttpServiceError> {
    service.stop(room).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn client_router(
    service: Arc<dyn crate::services::NativeRoomClientHttpService>,
    token: String,
) -> Result<Router, HttpServiceError> {
    protect(
        Router::new()
            .route("/native/room-client/join", post(join_guest))
            .route("/native/room-client/resume", post(resume_guest))
            .route("/native/room-client/recover", post(recover_guest))
            .route(
                "/native/room-client/saved",
                axum::routing::get(saved_guests),
            )
            .route("/native/room-client/postpone", post(postpone_guest))
            .layer(DefaultBodyLimit::max(4096))
            .layer(Extension(service)),
        token,
    )
}
async fn join_guest(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomClientHttpService>>,
    Json(request): Json<app_application::models::room_client::NativeGuestJoinRequest>,
) -> Result<Json<app_application::models::room_client::NativeGuestConnection>, HttpServiceError> {
    service.join(request).await.map(Json)
}
async fn resume_guest(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomClientHttpService>>,
    Json(request): Json<app_application::models::room_client::NativeGuestResumeRequest>,
) -> Result<Json<app_application::models::room_client::NativeGuestConnection>, HttpServiceError> {
    service.resume(request).await.map(Json)
}

async fn recover_guest(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomClientHttpService>>,
) -> Result<
    Json<Option<app_application::models::room_client::NativeGuestConnection>>,
    HttpServiceError,
> {
    service.recover().await.map(Json)
}

async fn saved_guests(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomClientHttpService>>,
) -> Result<
    Json<Vec<app_application::models::room_client::NativeGuestRecoveryChoice>>,
    HttpServiceError,
> {
    service.saved().await.map(Json)
}
async fn postpone_guest(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomClientHttpService>>,
) -> Result<StatusCode, HttpServiceError> {
    service.postpone().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn transport_router(
    service: Arc<dyn crate::services::NativeRoomTransportHttpService>,
    token: String,
) -> Result<Router, HttpServiceError> {
    protect(
        Router::new()
            .route("/native/room-client/execute", post(execute_room))
            .route("/native/room-client/stream", post(stream_room))
            .layer(DefaultBodyLimit::max(
                crate::routes::media_assets::MAX_UPLOAD_BODY_BYTES + 16 * 1024,
            ))
            .layer(Extension(service)),
        token,
    )
}
async fn execute_room(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomTransportHttpService>>,
    Json(request): Json<app_application::models::room_transport::NativeRoomTransportRequest>,
) -> Result<
    Json<app_application::models::room_transport::NativeRoomTransportResult>,
    HttpServiceError,
> {
    service.execute(request).await.map(Json)
}

async fn stream_room(
    Extension(service): Extension<Arc<dyn crate::services::NativeRoomTransportHttpService>>,
    Json(request): Json<app_application::models::room_transport::NativeRoomTransportRequest>,
) -> Result<axum::response::Response, HttpServiceError> {
    use axum::response::IntoResponse;
    use futures::TryStreamExt;
    let stream = service
        .stream(request)
        .await?
        .map_err(|_| std::io::Error::other("native_room_stream_interrupted"));
    Ok((
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        axum::body::Body::from_stream(stream),
    )
        .into_response())
}
