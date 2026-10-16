//! Main-only native file IO. No path, token or package bytes cross into the UI.
use super::{backend::BackendState, rooms::bundle_guest_target};
use crate::processes::control::ControlServer;
use app_application::models::{
    campaign_bundle::BundleKind,
    portable_campaign::{CampaignBundlePreview, CampaignImportResult, MAX_BUNDLE_FILE_BYTES},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;
use uuid::Uuid;
#[derive(Default)]
pub struct BundleFileState {
    prepared: Mutex<BTreeMap<Uuid, Vec<u8>>>,
    saved: Mutex<Option<PathBuf>>,
    lane: tokio::sync::Mutex<()>,
}
fn operation_store(
    window: &WebviewWindow,
) -> Result<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>, String> {
    window
        .app_handle()
        .store("bundle-operations.json")
        .map_err(|_| "native_bundle_state_failed".into())
}
fn pending_imports(window: &WebviewWindow) -> Result<BTreeMap<String, Uuid>, String> {
    operation_store(window)?
        .get("pending_imports_v1")
        .map(serde_json::from_value)
        .transpose()
        .map(|value| value.unwrap_or_default())
        .map_err(|_| "native_bundle_state_failed".into())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportChoice {
    operation_id: Uuid,
    preview: CampaignBundlePreview,
    file_name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    file_name: String,
    bytes: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Ru,
}
fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("desktop_window_scope_denied".into())
    }
}
fn owner(window: &WebviewWindow) -> Result<(u16, String), String> {
    let app = window.app_handle();
    let backend = app.state::<BackendState>();
    let port = backend
        .port
        .lock()
        .map_err(|_| "native_backend_unavailable")?
        .ok_or("native_backend_unavailable")?;
    Ok((port, app.state::<ControlServer>().token().to_owned()))
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| "native_http_unavailable".into())
}
async fn body(mut response: reqwest::Response) -> Result<Vec<u8>, String> {
    let success = response.status().is_success();
    let limit = if success { MAX_BUNDLE_FILE_BYTES } else { 4096 };
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "native_bundle_delivery_unknown")?
    {
        if bytes.len() + chunk.len() > limit {
            return Err("bundle_limit_exceeded".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if !success {
        let code = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/code")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            });
        return Err(code
            .filter(|code| code.starts_with("bundle_") && code.len() < 80)
            .unwrap_or_else(|| "native_bundle_request_failed".into()));
    }
    Ok(bytes)
}
fn copy(language: Language, key: &str) -> Result<String, String> {
    let text = match language {
        Language::En => include_str!("../../../src/locales/en/common.json"),
        Language::Ru => include_str!("../../../src/locales/ru/common.json"),
    };
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|catalog| {
            catalog
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .ok_or_else(|| "native_bundle_catalog_invalid".into())
}
async fn picker(
    window: &WebviewWindow,
    save: bool,
    language: Language,
) -> Result<Option<PathBuf>, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let mut builder = window
        .app_handle()
        .dialog()
        .file()
        .set_parent(window)
        .add_filter("Dungeon Master AI", &["dmai"])
        .set_title(copy(
            language,
            if save {
                "bundle_save_dialog"
            } else {
                "bundle_open_dialog"
            },
        )?);
    if save {
        builder = builder.set_file_name("campaign.dmai");
        builder.save_file(move |path| {
            let _ = send.send(path);
        });
    } else {
        builder.pick_file(move |path| {
            let _ = send.send(path);
        });
    }
    let selected = receive.await.map_err(|_| "native_bundle_dialog_failed")?;
    if selected.is_none() {
        log::info!("native bundle action=dialog_cancelled save={save}");
    }
    selected
        .map(|path| {
            path.into_path()
                .map_err(|_| "native_bundle_path_invalid".into())
        })
        .transpose()
}
fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path).map_err(|_| "native_bundle_read_failed")?;
    if !file
        .metadata()
        .map_err(|_| "native_bundle_read_failed")?
        .is_file()
    {
        return Err("native_bundle_path_invalid".into());
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_BUNDLE_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "native_bundle_read_failed")?;
    if bytes.len() > MAX_BUNDLE_FILE_BYTES {
        return Err("bundle_limit_exceeded".into());
    }
    Ok(bytes)
}
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_BUNDLE_FILE_BYTES {
        return Err("bundle_limit_exceeded".into());
    }
    let parent = path.parent().ok_or("native_bundle_path_invalid")?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "native_bundle_write_failed")?;
    file.write_all(bytes)
        .and_then(|()| file.as_file().sync_all())
        .map_err(|_| "native_bundle_write_failed")?;
    file.persist_noclobber(path).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            "native_bundle_target_exists"
        } else {
            "native_bundle_write_failed"
        }
    })?;
    Ok(())
}
#[tauri::command]
pub async fn desktop_bundle_export(
    window: WebviewWindow,
    campaign_id: Uuid,
    kind: BundleKind,
    language: Language,
) -> Result<Option<ExportResult>, String> {
    main_only(&window)?;
    if campaign_id.is_nil() {
        return Err("bundle_identity_invalid".into());
    }
    let app = window.app_handle();
    let state = app.state::<BundleFileState>();
    let _lane = state.lane.try_lock().map_err(|_| "native_bundle_busy")?;
    let target = bundle_guest_target(app, campaign_id, kind)?;
    let (port, token) = owner(&window)?;
    let response = if let Some(target) = target.as_ref() {
        client()?
            .get(format!(
                "http://{}/rooms/{}/dossier",
                target.address, target.room_id
            ))
            .header("x-room-member-id", target.member_id.to_string())
            .bearer_auth(&target.capability)
            .send()
            .await
    } else {
        client()?
            .post(format!(
                "http://127.0.0.1:{port}/native/campaigns/{campaign_id}/bundle/export"
            ))
            .bearer_auth(token)
            .json(&serde_json::json!({"kind":kind}))
            .send()
            .await
    }
    .map_err(|_| "native_bundle_request_failed")?;
    let bytes = body(response).await?;
    let Some(path) = picker(&window, true, language).await? else {
        return Ok(None);
    };
    let current_target = bundle_guest_target(app, campaign_id, kind)?;
    let identity =
        |target: &app_application::models::room_transport::NativeRoomTransportRequest| {
            (
                target.address,
                target.room_id,
                target.member_id,
                target.capability.clone(),
            )
        };
    if target.as_ref().map(identity) != current_target.as_ref().map(identity) {
        return Err("native_room_selection_changed".into());
    }
    let destination = path.clone();
    let total = bytes.len() as u64;
    tauri::async_runtime::spawn_blocking(move || write_file(&destination, &bytes))
        .await
        .map_err(|_| "native_bundle_write_failed")??;
    *state
        .saved
        .lock()
        .map_err(|_| "native_bundle_state_failed")? = Some(path.clone());
    log::info!("native bundle action=export_completed bytes={total}");
    Ok(Some(ExportResult {
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        bytes: total,
    }))
}
#[tauri::command]
pub async fn desktop_bundle_pick_import(
    window: WebviewWindow,
    language: Language,
) -> Result<Option<ImportChoice>, String> {
    main_only(&window)?;
    let app = window.app_handle();
    let state = app.state::<BundleFileState>();
    let _lane = state.lane.try_lock().map_err(|_| "native_bundle_busy")?;
    let Some(path) = picker(&window, false, language).await? else {
        return Ok(None);
    };
    let input = path.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || read_file(&input))
        .await
        .map_err(|_| "native_bundle_read_failed")??;
    let (port, token) = owner(&window)?;
    let response = client()?
        .post(format!(
            "http://127.0.0.1:{port}/native/campaigns/bundle/preview"
        ))
        .bearer_auth(token)
        .body(bytes.clone())
        .send()
        .await
        .map_err(|_| "native_bundle_request_failed")?;
    let preview: CampaignBundlePreview = serde_json::from_slice(&body(response).await?)
        .map_err(|_| "native_bundle_response_invalid")?;
    if preview.kind != BundleKind::FullCampaignBackup {
        return Err("bundle_not_restorable".into());
    }
    let digest = app_application::models::portable_campaign::PortableCampaignBundle::decode(&bytes)
        .map_err(|_| "native_bundle_response_invalid")?
        .integrity_sha256;
    let operation_id = pending_imports(&window)?
        .get(&digest)
        .copied()
        .unwrap_or_else(Uuid::new_v4);
    let mut prepared = state
        .prepared
        .lock()
        .map_err(|_| "native_bundle_state_failed")?;
    if prepared.len() >= 2 {
        return Err("native_bundle_busy".into());
    }
    prepared.insert(operation_id, bytes);
    Ok(Some(ImportChoice {
        operation_id,
        preview,
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    }))
}
#[tauri::command]
pub async fn desktop_bundle_confirm_import(
    window: WebviewWindow,
    operation_id: Uuid,
) -> Result<CampaignImportResult, String> {
    main_only(&window)?;
    let state = window.state::<BundleFileState>();
    let _lane = state.lane.try_lock().map_err(|_| "native_bundle_busy")?;
    let bytes = state
        .prepared
        .lock()
        .map_err(|_| "native_bundle_state_failed")?
        .get(&operation_id)
        .cloned()
        .ok_or("native_bundle_preview_expired")?;
    let (port, token) = owner(&window)?;
    let digest = app_application::models::portable_campaign::PortableCampaignBundle::decode(&bytes)
        .map_err(|_| "native_bundle_response_invalid")?
        .integrity_sha256;
    let store = operation_store(&window)?;
    let mut pending = pending_imports(&window)?;
    if pending.len() >= 16 && !pending.contains_key(&digest) {
        return Err("native_bundle_busy".into());
    }
    pending.insert(digest.clone(), operation_id);
    store.set(
        "pending_imports_v1",
        serde_json::to_value(&pending).map_err(|_| "native_bundle_state_failed")?,
    );
    store.save().map_err(|_| "native_bundle_state_failed")?;
    let response = client()?
        .post(format!(
            "http://127.0.0.1:{port}/native/campaigns/bundle/import"
        ))
        .bearer_auth(token)
        .header("x-import-operation-id", operation_id.to_string())
        .body(bytes)
        .send()
        .await
        .map_err(|_| "native_bundle_delivery_unknown")?;
    let result = serde_json::from_slice(&body(response).await?)
        .map_err(|_| "native_bundle_response_invalid")?;
    pending.remove(&digest);
    store.set(
        "pending_imports_v1",
        serde_json::to_value(&pending).map_err(|_| "native_bundle_state_failed")?,
    );
    store.save().map_err(|_| "native_bundle_state_failed")?;
    state
        .prepared
        .lock()
        .map_err(|_| "native_bundle_state_failed")?
        .remove(&operation_id);
    log::info!("native bundle action=import_completed");
    Ok(result)
}
#[tauri::command]
pub fn desktop_bundle_cancel(window: WebviewWindow, operation_id: Uuid) -> Result<(), String> {
    main_only(&window)?;
    let state = window.state::<BundleFileState>();
    let _lane = state.lane.try_lock().map_err(|_| "native_bundle_busy")?;
    let removed = window
        .state::<BundleFileState>()
        .prepared
        .lock()
        .map_err(|_| "native_bundle_state_failed")?
        .remove(&operation_id);
    if removed.is_some() {
        log::info!("native bundle action=import_preview_cancelled");
    }
    Ok(())
}
#[tauri::command]
pub fn desktop_bundle_reveal(window: WebviewWindow) -> Result<(), String> {
    main_only(&window)?;
    let state = window.state::<BundleFileState>();
    let path = state
        .saved
        .lock()
        .map_err(|_| "native_bundle_state_failed")?
        .clone()
        .ok_or("native_bundle_file_unavailable")?;
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|_| "native_bundle_reveal_failed".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_new_file_preserves_an_existing_destination_and_accepts_unicode_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Кампания с пробелами.dmai");
        write_file(&path, b"first").unwrap();
        assert_eq!(read_file(&path).unwrap(), b"first");
        assert_eq!(
            write_file(&path, b"replace"),
            Err("native_bundle_target_exists".into())
        );
        assert_eq!(read_file(&path).unwrap(), b"first");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn missing_destination_directory_and_nonfile_input_fail_without_partial_output() {
        let dir = tempfile::tempdir().unwrap();
        assert!(write_file(&dir.path().join("missing/output.dmai"), b"data").is_err());
        assert!(read_file(dir.path()).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    #[test]
    fn oversized_file_is_rejected_by_native_io() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.dmai");
        std::fs::File::create(&path)
            .unwrap()
            .set_len((MAX_BUNDLE_FILE_BYTES + 1) as u64)
            .unwrap();
        assert_eq!(read_file(&path), Err("bundle_limit_exceeded".into()));
    }
    #[test]
    #[ignore = "requires an owned directory with OS write permission denied"]
    fn os_denied_directory_leaves_no_output() {
        let dir = PathBuf::from(std::env::var_os("DMAI_BUNDLE_READONLY_TEST_DIRECTORY").unwrap());
        assert!(dir.is_dir());
        let path = dir.join("denied.dmai");
        assert_eq!(
            write_file(&path, b"fixture"),
            Err("native_bundle_write_failed".into())
        );
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), 0);
    }
}
