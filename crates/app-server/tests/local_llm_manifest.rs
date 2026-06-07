//! Smoke coverage for the M9-DM Task 14 endpoint pair:
//! - `GET /local-llm/manifest` returns the curated Qwen3.5 system catalog
//! - `POST /local-llm/active-model` accepts known ids and rejects unknown ids

use app_server::test_support::TestServer;

#[tokio::test]
async fn manifest_returns_gemma_and_qwen_entries() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/local-llm/manifest"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");

    let system = body["system"].as_array().expect("system is array");
    assert_eq!(system.len(), 7);
    let ids: Vec<&str> = system
        .iter()
        .map(|e| e["id"].as_str().expect("id is string"))
        .collect();
    assert_eq!(
        ids,
        vec![
            "qwen3-8b",
            "gemma-4-e2b",
            "gemma-4-e4b",
            "qwen3.5-0.8b",
            "qwen3.5-2b",
            "qwen3.5-4b",
            "qwen3.5-9b"
        ]
    );
    // User manifest is empty until HF search lands.
    assert_eq!(body["user"].as_array().unwrap().len(), 0);
    // Installed ids and download states default to empty for a fresh server.
    assert_eq!(body["installed_ids"].as_array().unwrap().len(), 0);
    assert!(body["download_states"].is_object());
}

#[tokio::test]
async fn manifest_includes_user_entries_added_via_hf() {
    // End-to-end proof that the HF-search write path and the picker read path
    // share one user_manifest.json: add a model via POST /hf/manifest/add, then
    // GET /local-llm/manifest and assert it appears under `user`. models_dir is
    // redirected to a unique temp dir so the shared temp manifest is untouched.
    let server = TestServer::start().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    let models_dir = tmp.path().join("models");
    std::fs::create_dir_all(&models_dir).expect("mkdir models");
    server.state.set_models_dir(models_dir);

    let client = reqwest::Client::new();
    let add = client
        .post(server.url("/hf/manifest/add"))
        .json(&serde_json::json!({
            "repo_id": "acme/Model-GGUF",
            "hf_filename": "model-q4_k_m.gguf",
            "arch": "qwen3",
            "quant": "gguf-q4_k_m",
            "size_gb": 2.0,
            "license": "apache-2.0",
            "display_name": "Acme Q4",
            "force": true
        }))
        .send()
        .await
        .expect("add request");
    assert_eq!(add.status(), 201, "add should return 201 Created");

    let resp = reqwest::get(server.url("/local-llm/manifest"))
        .await
        .expect("manifest request");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    let user = body["user"].as_array().expect("user is array");
    assert!(
        user.iter().any(|e| e["hf_repo"] == "acme/Model-GGUF"),
        "user-added model should reach the picker, got: {user:?}"
    );
}

#[tokio::test]
async fn active_model_accepts_known_id() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/local-llm/active-model"))
        .json(&serde_json::json!({ "id": "qwen3.5-2b" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn active_model_rejects_unknown_id() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/local-llm/active-model"))
        .json(&serde_json::json!({ "id": "made-up-model" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 400);
}
