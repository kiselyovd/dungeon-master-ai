//! Dedicated room transport; never exposes the local desktop administration API.
use crate::services::HttpServiceError;
use app_application::{
    models::room::PublicRoom,
    models::{identity::Revision, room_character::*},
    rooms::{JoinRoom, RoomError, RoomSecret, RoomService, RoomSubject},
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
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
        .route("/rooms/{room_id}/leave", post(leave_room))
        .route("/rooms/{room_id}/party", get(own_party))
        .route("/rooms/{room_id}/dossier", get(own_dossier))
        .route(
            "/rooms/{room_id}/media/assets",
            post(upload_portrait).layer(DefaultBodyLimit::max(
                crate::routes::media_assets::MAX_UPLOAD_BODY_BYTES,
            )),
        )
        .route(
            "/rooms/{room_id}/media/assets/{asset_id}",
            get(public_media_asset),
        )
        .route("/rooms/{room_id}/state", get(room_state))
        .route("/rooms/{room_id}/updates", get(room_updates))
        .route("/rooms/{room_id}/events", get(room_events))
        .route("/rooms/{room_id}/adventure/actions", post(adventure_action))
        .route("/rooms/{room_id}/rest", get(rest_status).post(rest_action))
        .route("/rooms/{room_id}/recovery", post(recovery_action))
        .route("/rooms/{room_id}/combat/{encounter_id}", get(room_combat))
        .route(
            "/rooms/{room_id}/combat/{encounter_id}/actions",
            post(act_combat),
        )
        .route(
            "/rooms/{room_id}/character-proposals",
            get(pending_proposals)
                .post(submit_character)
                .layer(DefaultBodyLimit::max(
                    app_application::models::campaign_state::MAX_PORTRAIT_URL_BYTES
                        + app_application::models::campaign_state::MAX_CHARACTER_METADATA_BYTES
                        + 64 * 1024,
                )),
        )
        .route(
            "/rooms/{room_id}/character-proposals/own",
            get(own_proposal),
        )
        .route(
            "/rooms/{room_id}/character-proposals/{proposal_id}/approve",
            post(approve_character),
        )
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomPortraitUpload {
    asset_id: Uuid,
    mime_type: String,
    data_b64: String,
}
async fn upload_portrait(
    State(service): State<Arc<RoomService>>,
    Path(room): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<RoomPortraitUpload>,
) -> Result<Json<app_application::models::media_asset::MediaReference>, HttpServiceError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let subject = subject(&service, room, &headers).await?;
    let limit = app_application::models::media_asset::MAX_MEDIA_ASSET_BYTES;
    if body.data_b64.len() > limit.div_ceil(3) * 4 {
        return Err(HttpServiceError::PayloadTooLarge {
            code: "media_asset_too_large",
        });
    };
    let bytes = STANDARD
        .decode(body.data_b64)
        .map_err(|_| HttpServiceError::BadRequest {
            code: "media_asset_invalid",
        })?;
    service
        .upload_portrait(&subject, body.asset_id, &body.mime_type, bytes)
        .await
        .map(Json)
        .map_err(crate::routes::media_assets::media_asset_error)
}
async fn public_media_asset(
    State(service): State<Arc<RoomService>>,
    Path((room, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HttpServiceError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let subject = subject(&service, room, &headers).await?;
    let asset = service
        .public_asset(&subject, id)
        .await
        .map_err(crate::routes::media_assets::media_asset_error)?;
    Ok(Json(
        serde_json::json!({"media_ref":asset.reference,"data_b64":STANDARD.encode(asset.bytes)}),
    ))
}
fn room_error(error: RoomError) -> HttpServiceError {
    match error {
        RoomError::ActionRejected => HttpServiceError::UnprocessableEntity {
            code: "room_action_rejected",
        },
        RoomError::Conflict => HttpServiceError::Conflict {
            code: "room_state_conflict",
        },
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaveRoomRequest {
    request_id: String,
    expected_epoch: u64,
}
async fn leave_room(
    State(service): State<Arc<RoomService>>,
    Path(room): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<LeaveRoomRequest>,
) -> Result<Json<app_application::models::room::RoomDepartureReceipt>, HttpServiceError> {
    let member = Uuid::parse_str(single_header(&headers, "x-room-member-id")?)
        .map_err(|_| room_error(RoomError::Denied))?;
    let capability = single_header(&headers, "authorization")?
        .strip_prefix("Bearer ")
        .ok_or_else(|| room_error(RoomError::Denied))?;
    let capability = RoomSecret::from_input(capability.into()).map_err(room_error)?;
    let subject = service
        .authenticate_departure(room, member, &capability)
        .await
        .map_err(room_error)?;
    service
        .leave(&subject, request.expected_epoch, &request.request_id)
        .await
        .map(Json)
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
    #[serde(default)]
    return_intent: Option<app_application::models::room::RoomReturnIntent>,
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
    let command = JoinRoom {
        room_id,
        join_id: request.join_id,
        display_name: request.display_name,
        invitation: RoomSecret::from_input(request.invitation).map_err(room_error)?,
        resume_proof: RoomSecret::from_input(request.resume_proof).map_err(room_error)?,
    };
    let grant = match request.return_intent {
        Some(intent) => service.return_join(command, intent).await,
        None => service.join(command).await,
    }
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
async fn own_dossier(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<([(header::HeaderName, &'static str); 2], Vec<u8>), HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .export_own_dossier(&subject)
        .await
        .map(|bytes| {
            (
                [
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (header::CACHE_CONTROL, "no-store"),
                ],
                bytes,
            )
        })
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CharacterRequest {
    proposal_id: Uuid,
    character_id: Uuid,
    character: serde_json::Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalRequest {
    expected_revision: u64,
    causation_id: String,
}
async fn submit_character(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CharacterRequest>,
) -> Result<Json<ProposalSummary>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .submit_character(
            &subject,
            SubmitRoomCharacter {
                proposal_id: request.proposal_id,
                character_id: request.character_id,
                character: request.character,
            },
        )
        .await
        .map(Json)
        .map_err(room_error)
}
async fn pending_proposals(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProposalSummary>>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .pending_proposals(&subject)
        .await
        .map(Json)
        .map_err(room_error)
}
async fn own_proposal(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Option<OwnCharacterProposal>>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .own_proposal(&subject)
        .await
        .map(Json)
        .map_err(room_error)
}
async fn approve_character(
    State(service): State<Arc<RoomService>>,
    Path((room_id, proposal_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<CharacterApproval>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .approve_character(
            &subject,
            ApproveRoomCharacter {
                proposal_id,
                expected_revision: Revision(request.expected_revision),
                causation_id: request.causation_id,
            },
        )
        .await
        .map(Json)
        .map_err(room_error)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CombatRequest {
    request_id: String,
    expected_revision: u64,
    action: app_application::agent::hero_intentions::HeroCombatIntent,
}
async fn room_combat(
    State(service): State<Arc<RoomService>>,
    Path((room_id, encounter)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<app_application::models::combat::CombatProjection>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .room_combat(&subject, encounter)
        .await
        .map(Json)
        .map_err(room_error)
}
async fn act_combat(
    State(service): State<Arc<RoomService>>,
    Path((room_id, encounter)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<CombatRequest>,
) -> Result<Json<app_application::models::room_combat::RoomCombatResult>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .act_combat(
            &subject,
            encounter,
            app_application::models::room_combat::RoomCombatCommand {
                request_id: request.request_id,
                expected_revision: request.expected_revision,
                action: request.action,
            },
        )
        .await
        .map(Json)
        .map_err(room_error)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateQuery {
    after: Option<u64>,
}
async fn room_state(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<app_application::models::room_state::RoomUpdate>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .updates(&subject, None)
        .await
        .map_err(room_error)?
        .map(Json)
        .ok_or(HttpServiceError::Internal {
            code: "room_unavailable",
        })
}
async fn room_updates(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<UpdateQuery>,
) -> Result<axum::response::Response, HttpServiceError> {
    use axum::response::IntoResponse;
    let subject = subject(&service, room_id, &headers).await?;
    Ok(
        match service
            .updates(&subject, query.after)
            .await
            .map_err(room_error)?
        {
            Some(update) => Json(update).into_response(),
            None => StatusCode::NO_CONTENT.into_response(),
        },
    )
}

async fn room_events(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<axum::response::Response, HttpServiceError> {
    use axum::response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    };
    let initial_subject = subject(&service, room_id, &headers).await?;
    let after = if headers.contains_key("last-event-id") {
        Some(
            single_header(&headers, "last-event-id")?
                .parse::<u64>()
                .map_err(|_| HttpServiceError::BadRequest {
                    code: "room_cursor_invalid",
                })?,
        )
    } else {
        None
    };
    let mut initial_subject = initial_subject;
    let mut attempts = 0;
    let initial = loop {
        match service.initial_updates(&initial_subject, after).await {
            Ok(update) => break update,
            Err(RoomError::Denied) if attempts < 2 => {
                attempts += 1;
                initial_subject = subject(&service, room_id, &headers).await?;
            }
            Err(error) => return Err(room_error(error)),
        }
    };
    let cursor = initial.cursor;
    let head = initial.head;
    let stream = futures::stream::unfold(
        (Some(initial), service, headers, room_id, cursor, head),
        |(initial, service, headers, room_id, mut cursor, mut head)| async move {
            if let Some(update) = initial {
                let event = Event::default()
                    .event("room_update")
                    .id(update.cursor.to_string())
                    .json_data(&update)
                    .ok()?;
                return Some((
                    Ok::<Event, std::convert::Infallible>(event),
                    (None, service, headers, room_id, cursor, head),
                ));
            }
            loop {
                if cursor == head {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
                // Fresh proof validation keeps remaining peers live across membership epochs.
                // Revoked/departed members still fail authentication before any new snapshot.
                let current = subject(&service, room_id, &headers).await.ok()?;
                match service.updates(&current, Some(cursor)).await {
                    Ok(Some(update)) => {
                        cursor = update.cursor;
                        head = update.head;
                        let event = Event::default()
                            .event("room_update")
                            .id(cursor.to_string())
                            .json_data(&update)
                            .ok()?;
                        return Some((
                            Ok::<Event, std::convert::Infallible>(event),
                            (None, service, headers, room_id, cursor, head),
                        ));
                    }
                    Ok(None) => {
                        head = cursor;
                    }
                    Err(RoomError::Denied) => {
                        // Membership can commit between authentication and the read.
                        // Authenticate again on the next bounded-rate poll; revoked peers
                        // fail before any new state is emitted.
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                    Err(_) => return None,
                }
            }
        },
    );
    Ok(Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(1))
                .text("room"),
        )
        .into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdventureRequest {
    request_id: String,
    expected_revision: u64,
    action: app_application::models::adventure::Action,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestRequest {
    request_id: String,
    expected_revision: u64,
    intent: app_application::models::rest::RestIntent,
}
async fn adventure_action(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AdventureRequest>,
) -> Result<Json<app_application::models::room_world::RoomWorldReceipt>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .adventure(
            &subject,
            app_application::models::room_world::RoomAdventureCommand {
                request_id: request.request_id,
                expected_revision: Revision(request.expected_revision),
                action: request.action,
            },
        )
        .await
        .map(Json)
        .map_err(room_error)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryRequest {
    request_id: String,
    expected_revision: u64,
    intent: app_application::models::room_recovery::RecoveryIntent,
}
async fn recovery_action(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<RecoveryRequest>,
) -> Result<Json<app_application::models::room_recovery::RoomRecoveryReceipt>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .recover_hero(
            &subject,
            app_application::models::room_recovery::RoomRecoveryCommand {
                request_id: request.request_id,
                expected_revision: Revision(request.expected_revision),
                intent: request.intent,
            },
        )
        .await
        .map(Json)
        .map_err(room_error)
}
async fn rest_status(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<app_application::models::rest::RestStatus>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .rest_status(&subject)
        .await
        .map(Json)
        .map_err(room_error)
}
async fn rest_action(
    State(service): State<Arc<RoomService>>,
    Path(room_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<RestRequest>,
) -> Result<Json<app_application::models::room_world::RoomWorldReceipt>, HttpServiceError> {
    let subject = subject(&service, room_id, &headers).await?;
    service
        .rest(
            &subject,
            app_application::models::room_world::RoomRestCommand {
                request_id: request.request_id,
                expected_revision: Revision(request.expected_revision),
                intent: request.intent,
            },
        )
        .await
        .map(Json)
        .map_err(room_error)
}

/// Optional shared-DM transport, composed with the same authenticated room service.
/// Provider/settings authority is exclusively supplied by the host composition.
pub fn router_with_dm(
    service: Arc<RoomService>,
    dm: Arc<dyn crate::services::RoomDmHttpService>,
) -> Router {
    router(service.clone()).merge(
        Router::new()
            .route("/rooms/{room_id}/dm/turn", post(dm_turn))
            .route("/rooms/{room_id}/dm/recovery", get(dm_recovery))
            .layer(axum::Extension(dm))
            .layer(DefaultBodyLimit::max(32 * 1024))
            .layer(middleware::from_fn(no_store))
            .with_state(service),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DmTurnRequest {
    #[serde(default)]
    clarification: Option<app_application::agent::human_commands::HumanCommandClarificationReply>,
    request_id: Uuid,
    player_message: String,
    #[serde(default)]
    resume: bool,
}
async fn dm_recovery(
    State(service): State<Arc<RoomService>>,
    axum::Extension(dm): axum::Extension<Arc<dyn crate::services::RoomDmHttpService>>,
    Path(room): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Option<app_application::agent::recovery::AgentRecoveryProjection>>, HttpServiceError>
{
    let verified = subject(&service, room, &headers).await?;
    Ok(Json(dm.recovery(verified).await?))
}
async fn dm_turn(
    State(service): State<Arc<RoomService>>,
    axum::Extension(dm): axum::Extension<Arc<dyn crate::services::RoomDmHttpService>>,
    Path(room): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<DmTurnRequest>,
) -> Result<Response, HttpServiceError> {
    use axum::response::{
        sse::{KeepAlive, Sse},
        IntoResponse,
    };
    use futures::StreamExt;
    let verified = subject(&service, room, &headers).await?;
    if input.request_id.is_nil()
        || input.player_message.trim().is_empty()
        || input.player_message.len() > 16384
    {
        return Err(HttpServiceError::BadRequest {
            code: "room_dm_request_invalid",
        });
    }
    let events = dm
        .turn_with_clarification(
            verified,
            input.request_id,
            input.player_message,
            input.resume,
            input.clarification,
        )
        .await?;
    let stream = events
        .map(|event| Ok::<_, std::convert::Infallible>(crate::sse::agent_event_to_sse(event)));
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response();
    response
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("close"));
    Ok(response)
}

/// An enabled public listener exposes exactly the room the native host opened.
pub fn router_for_room(
    service: Arc<RoomService>,
    dm: Arc<dyn crate::services::RoomDmHttpService>,
    room_id: Uuid,
) -> Router {
    router_with_dm(service, dm).layer(middleware::from_fn(
        move |request: axum::extract::Request, next: Next| async move {
            let mut parts = request.uri().path().split('/');
            let matches_room = parts.next() == Some("")
                && parts.next() == Some("rooms")
                && parts.next().and_then(|id| Uuid::parse_str(id).ok()) == Some(room_id);
            if !matches_room {
                return axum::response::IntoResponse::into_response(StatusCode::NOT_FOUND);
            }
            next.run(request).await
        },
    ))
}
