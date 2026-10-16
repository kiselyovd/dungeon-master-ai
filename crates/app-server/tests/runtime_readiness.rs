use app_application::{
    models::local_models::{RuntimeStartRequest, RuntimeStatus},
    ports::runtime::{RuntimeControl, RuntimeError},
};
use app_server::test_support::TestServer;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct RecordingRuntime(Mutex<Vec<RuntimeStartRequest>>);
#[async_trait::async_trait]
impl RuntimeControl for RecordingRuntime {
    async fn start(&self, request: RuntimeStartRequest) -> Result<RuntimeStatus, RuntimeError> {
        self.0.lock().unwrap().push(request);
        Ok(RuntimeStatus::default())
    }
    async fn stop(&self) -> Result<RuntimeStatus, RuntimeError> {
        Ok(RuntimeStatus::default())
    }
    async fn status(&self) -> Result<RuntimeStatus, RuntimeError> {
        Ok(RuntimeStatus::default())
    }
}
#[tokio::test]
async fn starting_chat_only_requests_enabled_local_media_from_the_native_owner() {
    use adapter_media::{
        image::{bundled::BundledImageProvider, stub::LocalImageSidecarProvider},
        video::sidecar::LocalVideoSidecarProvider,
    };
    let server = TestServer::start().await;
    server.state.set_local_mode_config(Default::default());
    server.state.clear_image_provider();
    server.state.clear_video_provider();
    let runtime = Arc::new(RecordingRuntime::default());
    server.state.set_runtime_control(runtime.clone());
    let client = reqwest::Client::new();
    for (image, video) in [(false, false), (false, false), (true, true)] {
        if runtime.0.lock().unwrap().len() == 1 {
            server
                .state
                .set_image_provider(Arc::new(BundledImageProvider::new()));
        }
        if image {
            server
                .state
                .set_image_provider(Arc::new(LocalImageSidecarProvider::new(
                    "http://127.0.0.1:1",
                )));
            server
                .state
                .set_video_provider(Arc::new(LocalVideoSidecarProvider::new(
                    "http://127.0.0.1:1",
                )));
        }
        assert_eq!(
            client
                .post(server.url("/local/runtime/start"))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        let recorded = runtime.0.lock().unwrap();
        let request = recorded.last().unwrap();
        assert!(request
            .llm_args
            .windows(2)
            .any(|args| args == ["--max-seq-len", "8192"]));
        assert_eq!(request.enable_image, image);
        assert_eq!(request.enable_video, video);
    }
}

#[tokio::test]
async fn context_choice_is_bounded_and_reaches_real_native_process_arguments() {
    let server = TestServer::start().await;
    let runtime = Arc::new(RecordingRuntime::default());
    server.state.set_runtime_control(runtime.clone());
    let client = reqwest::Client::new();
    for context in [0, 511, 65537] {
        assert_eq!(client.post(server.url("/local-mode/config")).json(&serde_json::json!({"selected_llm":"gemma4_e2b","vram_strategy":"auto-swap","context_window":context})).send().await.unwrap().status(), 400);
    }
    assert!(runtime.0.lock().unwrap().is_empty());
    assert_eq!(client.post(server.url("/local-mode/config")).json(&serde_json::json!({"selected_llm":"gemma4_e2b","vram_strategy":"auto-swap","context_window":4096})).send().await.unwrap().status(), 200);
    assert_eq!(
        client
            .post(server.url("/local/runtime/start"))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let recorded = runtime.0.lock().unwrap();
    assert!(recorded[0]
        .llm_args
        .windows(2)
        .any(|args| args == ["--max-seq-len", "4096"]));
}
