pub mod backend;
pub mod control;
pub mod media_runtime;
pub mod model_runtime;

use std::sync::{Arc, Mutex};

use app_application::models::local_models::{RuntimeStartRequest, RuntimeState, RuntimeStatus};
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

struct RuntimeInner {
    generation: u64,
    model_child: Option<CommandChild>,
    media_child: Option<CommandChild>,
    status: RuntimeStatus,
}

#[derive(Clone)]
pub struct RuntimeProcesses {
    inner: Arc<Mutex<RuntimeInner>>,
}

impl Default for RuntimeProcesses {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                generation: 0,
                model_child: None,
                media_child: None,
                status: RuntimeStatus::default(),
            })),
        }
    }
}

impl RuntimeProcesses {
    pub fn status(&self) -> RuntimeStatus {
        self.inner
            .lock()
            .expect("runtime process lock poisoned")
            .status
            .clone()
    }

    pub async fn start(&self, app: &AppHandle, request: RuntimeStartRequest) -> RuntimeStatus {
        self.stop().await;
        let generation = {
            let mut inner = self.inner.lock().expect("runtime process lock poisoned");
            inner.generation = inner.generation.wrapping_add(1);
            inner.status = RuntimeStatus {
                state: RuntimeState::Starting,
                model_id: Some(request.model_id.clone()),
                image_enabled: request.enable_image,
                video_enabled: request.enable_video,
                failure_code: None,
                llm_port: None,
                media_port: None,
            };
            inner.generation
        };

        let llm_port = match model_runtime::free_loopback_port() {
            Ok(port) => port,
            Err(_) => return self.fail(generation, "llm_port_unavailable"),
        };
        let llm_args = model_runtime::with_runtime_port(request.llm_args, llm_port);
        let (model_events, model_child) = match app
            .shell()
            .sidecar("mistralrs-server")
            .and_then(|command| command.args(llm_args).spawn())
        {
            Ok(pair) => pair,
            Err(error) => {
                log::error!("model runtime spawn failed: {error}");
                return self.fail(generation, "llm_spawn_failed");
            }
        };
        log::info!(
            "process_role=model-runtime pid={} lifecycle=spawned port={llm_port}",
            model_child.pid()
        );
        self.monitor_child(app.clone(), model_events, generation, "model-runtime");
        self.inner
            .lock()
            .expect("runtime process lock poisoned")
            .model_child = Some(model_child);

        if !model_runtime::probe(llm_port).await {
            self.stop().await;
            return self.fail(generation.wrapping_add(1), "llm_health_failed");
        }

        let mut media_port = None;
        if request.enable_image || request.enable_video {
            let port = match media_runtime::free_loopback_port() {
                Ok(port) => port,
                Err(_) => return self.degrade(generation, llm_port, "media_port_unavailable"),
            };
            let Some(weights_dir) = request.weights_dir else {
                return self.degrade(generation, llm_port, "media_weights_missing");
            };
            let (media_events, media_child) = match app
                .shell()
                .sidecar("dmai-image-sidecar")
                .and_then(|command| {
                    command
                        .args(media_runtime::args(port, &weights_dir))
                        .spawn()
                }) {
                Ok(pair) => pair,
                Err(error) => {
                    log::error!("media runtime spawn failed: {error}");
                    return self.degrade(generation, llm_port, "media_spawn_failed");
                }
            };
            log::info!(
                "process_role=media-runtime pid={} lifecycle=spawned port={port}",
                media_child.pid()
            );
            self.monitor_child(app.clone(), media_events, generation, "media-runtime");
            self.inner
                .lock()
                .expect("runtime process lock poisoned")
                .media_child = Some(media_child);
            if !media_runtime::probe(port).await {
                return self.degrade(generation, llm_port, "media_health_failed");
            }
            media_port = Some(port);
        }

        let status = RuntimeStatus {
            state: RuntimeState::Running,
            model_id: Some(request.model_id),
            image_enabled: request.enable_image,
            video_enabled: request.enable_video,
            failure_code: None,
            llm_port: Some(llm_port),
            media_port,
        };
        self.set_status_if_current(generation, status.clone());
        log::info!("process_role=runtime lifecycle=ready");
        status
    }

    pub async fn stop(&self) -> RuntimeStatus {
        let (media, model) = {
            let mut inner = self.inner.lock().expect("runtime process lock poisoned");
            inner.generation = inner.generation.wrapping_add(1);
            inner.status = RuntimeStatus::default();
            (inner.media_child.take(), inner.model_child.take())
        };
        if let Some(child) = media {
            let _ = child.kill();
        }
        if let Some(child) = model {
            let _ = child.kill();
        }
        log::info!("process_role=runtime lifecycle=stopped");
        RuntimeStatus::default()
    }

    fn fail(&self, generation: u64, code: &'static str) -> RuntimeStatus {
        let current = self.status();
        let status = RuntimeStatus {
            state: RuntimeState::Failed,
            failure_code: Some(code.to_owned()),
            ..current
        };
        self.set_status_if_current(generation, status.clone());
        status
    }

    fn degrade(&self, generation: u64, llm_port: u16, code: &'static str) -> RuntimeStatus {
        let current = self.status();
        let status = RuntimeStatus {
            state: RuntimeState::Degraded,
            failure_code: Some(code.to_owned()),
            llm_port: Some(llm_port),
            ..current
        };
        self.set_status_if_current(generation, status.clone());
        status
    }

    fn set_status_if_current(&self, generation: u64, status: RuntimeStatus) {
        let mut inner = self.inner.lock().expect("runtime process lock poisoned");
        if inner.generation == generation {
            inner.status = status;
        }
    }

    fn is_current(&self, generation: u64) -> bool {
        self.inner
            .lock()
            .expect("runtime process lock poisoned")
            .generation
            == generation
    }

    fn monitor_child(
        &self,
        app: AppHandle,
        mut events: tokio::sync::mpsc::Receiver<CommandEvent>,
        generation: u64,
        role: &'static str,
    ) {
        let processes = self.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = events.recv().await {
                match event {
                    CommandEvent::Terminated(status) => {
                        let code = status.code;
                        log::warn!("process_role={role} lifecycle=exited exit_code={code:?}");
                        if processes.is_current(generation) {
                            let failed = processes.fail(generation, "runtime_child_exited");
                            let _ = app.emit("runtime-exited", failed);
                        }
                        break;
                    }
                    CommandEvent::Error(error) => {
                        log::error!("process_role={role} lifecycle=error error={error}");
                        if processes.is_current(generation) {
                            let failed = processes.fail(generation, "runtime_child_error");
                            let _ = app.emit("runtime-exited", failed);
                        }
                        break;
                    }
                    CommandEvent::Stderr(bytes) => {
                        log::debug!("process_role={role} stderr_bytes={}", bytes.len());
                    }
                    _ => {}
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generation_cannot_replace_newer_status() {
        let processes = RuntimeProcesses::default();
        {
            let mut inner = processes.inner.lock().unwrap();
            inner.generation = 2;
        }
        processes.set_status_if_current(
            1,
            RuntimeStatus {
                state: RuntimeState::Failed,
                failure_code: Some("stale".into()),
                ..RuntimeStatus::default()
            },
        );
        assert_eq!(processes.status().state, RuntimeState::Stopped);
    }
}
