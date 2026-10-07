//! Dedicated room transport; never exposes the local desktop administration API.
use crate::services::HttpServiceError;
use app_application::{
    models::room::PublicRoom,
    rooms::{JoinRoom, RoomError, RoomSecret, RoomService, RoomSubject},
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Composed separately from the desktop router; does not open a listener.
pub fn router(service: Arc<RoomService>) -> Router {
    Router::new()
        .route("/rooms/{room_id}", get(current_room).delete(close_room))
        .route("/rooms/{room_id}/join", post(join_room))
        .route("/rooms/{room_id}/party", get(own_party))
        .route("/rooms/{room_id}/invitation", post(invite))
        .route("/rooms/{room_id}/members/{member_id}", delete(revoke))
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::from_fn(no_store))
        .with_state(service)
}
async fn no_store(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
fn room_error(error: RoomError) -> HttpServiceError {
    match error {
        RoomError::Denied => HttpServiceError::Unauthorized {
            code: "room_access_denied",
        },
        RoomError::InvalidInput => HttpServiceError::BadRequest {
            code: "room_input_invalid",
        },
        RoomError::Repository | RoomError::Secrets => HttpServiceError::Internal {
            code: "room_unavailable",
        },
    }
}
fn single_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, HttpServiceError> {
    let mut values = headers.get_all(name).iter();
    let value = values
        .next()
        .filter(|_| values.next().is_none())
        .and_then(|value| value.to_str().ok());
    value.ok_or_else(|| room_error(RoomError::Denied))
}
async fn subject(
    service: &RoomService,
    room: Uuid,
    headers: &HeaderMap,
) -> Result<RoomSubject, HttpServiceError> {
    let member = Uuid::parse_str(single_header(headers, "x-room-member-id")?)
        .map_err(|_| room_error(RoomError::Denied))?;
    let capability = single_header(headers, "authorization")?
        .strip_prefix("Bearer ")
        .ok_or_else(|| room_error(RoomError::Denied))?;
    let capability = RoomSecret::from_input(capability.into()).map_err(room_error)?;
    service
        .authenticate(room, member, &capability)
        .await
        .map_err(room_error)
}
// Secret-bearing DTOs deliberately have no Debug implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JoinRequest {
    join_id: Uuid,
    display_name: String,
    invitation: String,
    resume_proof: String,
}
#[derive(Serialize)]
struct JoinResponse {
    room: PublicRoom,
    member_id: Uuid,
    capability: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvitationRequest {
    target_member_id: Option<Uuid>,
}
#[derive(Serialize)]
struct InvitationResponse {
    invitation: String,
}
async fn join_room(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    Json(request): Json<JoinRequest>,
) -> Result<Json<JoinResponse>, HttpServiceError> {
    let grant = service
        .join(JoinRoom {
            room_id,
            join_id: request.join_id,
            display_name: request.display_name,
            invitation: RoomSecret::from_input(request.invitation).map_err(room_error)?,
            resume_proof: RoomSecret::from_input(request.resume_proof).map_err(room_error)?,
        })
        .await
        .map_err(room_error)?;
    Ok(Json(JoinResponse {
        room: grant.room,
        member_id: grant.member_id,
        capability: grant.capability.expose().into(),
    }))
}
async fn current_room(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PublicRoom>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .current_room(&subject)
        .await
        .map(Json)
        .map_err(room_error)
}

async fn own_party(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Option<app_application::models::party::PartyView>>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .own_party(&subject)
        .await
        .map(Json)
        .map_err(room_error)
}
async fn invite(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<InvitationRequest>,
) -> Result<Json<InvitationResponse>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    let invitation = match request.target_member_id {
        Some(member) => service.invite_member(&subject, member).await,
        None => service.rotate_invitation(&subject).await,
    }
    .map_err(room_error)?;
    Ok(Json(InvitationResponse {
        invitation: invitation.expose().into(),
    }))
}
async fn revoke(
    State(service): State<Arc<RoomService>>,
    Path((room_id, member_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .revoke(&subject, member_id)
        .await
        .map_err(room_error)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn close_room(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service.close(&subject).await.map_err(room_error)?;
    Ok(StatusCode::NO_CONTENT)
}
