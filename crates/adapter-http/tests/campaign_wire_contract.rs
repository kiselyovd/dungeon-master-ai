use adapter_http::routes::campaigns::{
    validate_create_campaign, CampaignResponse, CreateCampaignRequest,
};
use adapter_http::services::HttpServiceError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

#[test]
fn exact_collaborative_campaign_request_and_response_are_stable() {
    let request: CreateCampaignRequest = serde_json::from_value(json!({
        "schema_version": 1,
        "ruleset_id": "srd-5.2.1",
        "title": "Серебряная долина",
        "premise": "Древняя дорога проснулась",
        "seed": { "kind": "collaborative", "answers": { "genre": ["classic_fantasy"] } }
    }))
    .expect("campaign request must deserialize");

    let command = validate_create_campaign(request).expect("valid request");
    assert_eq!(command.title, "Серебряная долина");
    assert_eq!(command.premise, "Древняя дорога проснулась");

    let response: CampaignResponse = serde_json::from_value(json!({
        "id": "62ba8579-904b-4460-a68a-df6268ed404a",
        "schema_version": 1,
        "ruleset_id": "srd-5.2.1",
        "title": "Серебряная долина",
        "premise": "Древняя дорога проснулась",
        "seed": { "kind": "collaborative", "answers": { "genre": ["classic_fantasy"] } },
        "last_session_id": "b5271021-b82c-40c4-aa9f-554af0d79b22",
        "revision": 1,
        "created_at": "2026-08-13T03:21:00.000Z",
        "updated_at": "2026-08-13T03:21:00.000Z"
    }))
    .expect("campaign response must deserialize");

    assert_eq!(
        serde_json::to_value(response).expect("campaign response must serialize"),
        json!({
            "id": "62ba8579-904b-4460-a68a-df6268ed404a",
            "schema_version": 1,
            "ruleset_id": "srd-5.2.1",
            "title": "Серебряная долина",
            "premise": "Древняя дорога проснулась",
            "seed": { "kind": "collaborative", "answers": { "genre": ["classic_fantasy"] } },
            "last_session_id": "b5271021-b82c-40c4-aa9f-554af0d79b22",
            "revision": 1,
            "created_at": "2026-08-13T03:21:00.000Z",
            "updated_at": "2026-08-13T03:21:00.000Z"
        })
    );
}

#[test]
fn invalid_seed_values_are_rejected_without_echoing_input() {
    let request: CreateCampaignRequest = serde_json::from_value(json!({
        "schema_version": 1,
        "ruleset_id": "srd-5.2.1",
        "title": "Серебряная долина",
        "premise": "Древняя дорога проснулась",
        "seed": { "kind": "collaborative", "answers": { "genre": [] } }
    }))
    .expect("well-shaped request must deserialize");

    assert_eq!(
        validate_create_campaign(request),
        Err(HttpServiceError::UnprocessableEntity {
            code: "campaign_seed_invalid"
        })
    );
}

#[tokio::test]
async fn conflict_and_validation_errors_have_safe_wire_bodies() {
    for (error, status, code) in [
        (
            HttpServiceError::Conflict {
                code: "campaign_conflict",
            },
            StatusCode::CONFLICT,
            "campaign_conflict",
        ),
        (
            HttpServiceError::UnprocessableEntity {
                code: "campaign_seed_invalid",
            },
            StatusCode::UNPROCESSABLE_ENTITY,
            "campaign_seed_invalid",
        ),
    ] {
        let response = error.into_response();
        assert_eq!(response.status(), status);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("bounded error body");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("JSON error body"),
            json!({ "error": { "code": code, "message": code } })
        );
    }
}
