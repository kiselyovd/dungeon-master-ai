//! Sub-task #8 / Task 16: license-check smoke test.
//!
//! Exercises `HfClient::check_license` against a mocked HF API. The 200 and
//! 403 branches are the only two the UI actually distinguishes (accepted vs
//! gated-not-accepted); 401/404 surface as `HfError` and are covered by the
//! types' own tests.

use app_server::hf::client::HfClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn license_accepted_when_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/models/Qwen/Qwen3-4B"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "Qwen/Qwen3-4B",
                "gated": false,
                "tags": ["qwen3"],
                "siblings": []
            })),
        )
        .mount(&server)
        .await;

    let client = HfClient::new_with_base(Some("hf_test".into()), server.uri());
    let st = client.check_license("Qwen/Qwen3-4B").await.unwrap();
    assert!(st.accepted);
    assert!(!st.gated);
}

#[tokio::test]
async fn license_not_accepted_when_403() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/models/meta-llama/Llama-3-8B"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let client = HfClient::new_with_base(Some("hf_test".into()), server.uri());
    let st = client.check_license("meta-llama/Llama-3-8B").await.unwrap();
    assert!(st.gated);
    assert!(!st.accepted);
}
