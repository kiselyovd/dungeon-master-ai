//! Axum inbound adapter and stable wire mappers.

pub mod routes;
pub mod services;
pub mod sse;

use axum::extract::MatchedPath;
use axum::extract::{DefaultBodyLimit, Extension};
use axum::http::{HeaderName, Request};
use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{
    MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use services::HttpServices;

/// Build the complete stable HTTP surface from an immutable service bundle.
/// No concrete database, provider, vault, or process type crosses this boundary.
pub fn router(services: HttpServices) -> Router {
    let router = Router::new()
        .route("/health", get(routes::health::health))
        .route("/agent/turn", post(routes::agent::post_agent_turn))
        .route("/chat", post(routes::chat::chat))
        .route("/providers", get(routes::settings::get_providers))
        .route("/combat/start", post(routes::combat::post_combat_start))
        .route("/combat/action", post(routes::combat::post_combat_action))
        .route("/combat/end", post(routes::combat::post_combat_end))
        .route("/journal", get(routes::journal::get_journal))
        .route("/npcs", get(routes::npc::get_npcs))
        .route("/srd/races", get(routes::srd::get_races))
        .route("/srd/classes", get(routes::srd::get_classes))
        .route("/srd/backgrounds", get(routes::srd::get_backgrounds))
        .route("/srd/spells", get(routes::srd::get_spells))
        .route("/srd/equipment", get(routes::srd::get_equipment))
        .route("/srd/feats", get(routes::srd::get_feats))
        .route(
            "/srd/weapon-properties",
            get(routes::srd::get_weapon_properties),
        )
        .route(
            "/character/assist",
            post(routes::character_assist::post_character_assist),
        )
        .route(
            "/sessions/{session_id}/messages",
            get(routes::messages::list_messages),
        )
        .route(
            "/sessions/{session_id}/saves",
            get(routes::saves::list_saves).post(routes::saves::create_save),
        )
        .route(
            "/sessions/{session_id}/saves/quick",
            post(routes::saves::quick_save),
        )
        .route(
            "/saves/{save_id}",
            get(routes::saves::get_save)
                .put(routes::saves::update_save)
                .delete(routes::saves::delete_save),
        )
        .route(
            "/saves/{save_id}/restore",
            post(routes::saves::restore_save),
        )
        .route("/providers/catalog", get(routes::providers::get_catalog))
        .route("/providers/{id}/caps", get(routes::providers::get_caps))
        .route(
            "/providers/discover",
            post(routes::providers::post_discover),
        )
        .route("/settings/v2", post(routes::settings::post_settings_v2))
        .route("/image/generate", post(routes::image::post_image_generate))
        .route("/video/generate", post(routes::video::post_video_generate))
        .route(
            "/local-llm/manifest",
            get(routes::local_control::get_manifest),
        )
        .route(
            "/local-llm/active-model",
            post(routes::local_control::set_active_model),
        )
        .route(
            "/local-llm/download/{model_id}",
            post(routes::local_control::start_download),
        )
        .route(
            "/local-llm/model/{model_id}",
            delete(routes::local_control::cancel_or_delete),
        )
        .route(
            "/local-llm/download-events",
            get(routes::local_control::download_events),
        )
        .route(
            "/hf/token",
            post(routes::local_control::post_token).delete(routes::local_control::delete_token),
        )
        .route(
            "/hf/token/status",
            get(routes::local_control::get_token_status),
        )
        .route("/hf/search", get(routes::local_control::search))
        .route(
            "/hf/model/license/{*repo_id}",
            get(routes::local_control::license_check),
        )
        .route(
            "/hf/manifest/add",
            post(routes::local_control::add_manifest),
        )
        .route(
            "/hf/manifest/{id}",
            delete(routes::local_control::delete_manifest),
        );

    #[cfg(feature = "with-local-runtime")]
    let router = router
        .route(
            "/local-mode/config",
            get(routes::local_control::get_config).post(routes::local_control::post_config),
        )
        .route(
            "/local/download/{id}",
            post(routes::local_control::post_local_download)
                .delete(routes::local_control::delete_local_download),
        )
        .route(
            "/local/download/{id}/progress",
            get(routes::local_control::download_progress),
        )
        .route(
            "/local/runtime/start",
            post(routes::local_control::runtime_start),
        )
        .route(
            "/local/runtime/stop",
            post(routes::local_control::runtime_stop),
        )
        .route(
            "/local/runtime/status",
            get(routes::local_control::runtime_status),
        );

    let request_id_header = HeaderName::from_static("x-request-id");
    let trace = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<axum::body::Body>| {
            let route = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str)
                .unwrap_or("unmatched");
            let request_id = request
                .extensions()
                .get::<RequestId>()
                .and_then(|value| value.header_value().to_str().ok())
                .unwrap_or("missing");
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                route,
                request_id,
            )
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    router
        .layer(Extension(services))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .layer(trace)
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
}
