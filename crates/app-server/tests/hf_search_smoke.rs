//! Sub-task #8: HF search via wiremock.

use app_server::hf::client::HfClient;
use app_server::hf::types::{HfSearchQuery, HfSort};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_qwen3_returns_models() {
    let server = MockServer::start().await;

    let body = serde_json::json!([
        {
            "id": "Qwen/Qwen3-4B-Thinking-2507",
            "modelId": "Qwen/Qwen3-4B-Thinking-2507",
            "likes": 100,
            "downloads": 5000,
            "gated": false,
            "tags": ["text-generation", "qwen3", "license:apache-2.0"],
            "lastModified": "2026-04-01T00:00:00.000Z",
            "siblings": [
                { "rfilename": "qwen3-4b-thinking-2507-q4_k_m.gguf", "size": 2_300_000_000_u64 },
                { "rfilename": "qwen3-4b-thinking-2507-q8_0.gguf", "size": 4_600_000_000_u64 }
            ]
        }
    ]);

    Mock::given(method("GET"))
        .and(path("/api/models"))
        .and(query_param("search", "qwen3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = HfClient::new_with_base(None, server.uri());
    let results = client
        .search(HfSearchQuery {
            q: "qwen3".into(),
            arch: None,
            quant: None,
            size: None,
            license: None,
            sort: HfSort::Downloads,
        })
        .await
        .expect("search ok");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].repo_id, "Qwen/Qwen3-4B-Thinking-2507");
    assert!(!results[0].gated);
    assert_eq!(results[0].siblings.len(), 2);
}

#[tokio::test]
async fn compat_filters_unsupported_arch() {
    use app_server::hf::compat;
    assert!(compat::is_compat_arch("qwen3"));
    assert!(compat::is_compat_arch("llama3"));
    assert!(!compat::is_compat_arch("falcon"));
}

#[tokio::test]
async fn compat_filters_quant_filenames() {
    use app_server::hf::compat;
    assert!(compat::is_compat_quant("model-q4_k_m.gguf"));
    assert!(compat::is_compat_quant("model.safetensors"));
    assert!(!compat::is_compat_quant("model-q2_k.gguf"));
    assert!(!compat::is_compat_quant("README.md"));
}
