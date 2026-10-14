//! Native notification policy owns durable dedup, OS focus and delivery errors.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const STORE: &str = "notifications.json";
const MAX_SERIES: usize = 4096;
#[cfg(windows)]
#[path = "notifications/windows.rs"]
mod windows_delivery;
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    OwnTurn,
    RuntimeReady,
    DownloadReady,
    ConnectionFailed,
}
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Chat,
    Downloads,
    Connection,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Event {
    pub kind: Kind,
    pub scope_id: Uuid,
    pub event_id: String,
    pub target: Target,
}
#[derive(Default)]
pub struct NotificationState {
    game_scope: Mutex<Option<Uuid>>,
    lane: Mutex<()>,
    activations: std::sync::atomic::AtomicU64,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Granted,
    Denied,
    Unknown,
    #[cfg(not(windows))]
    Unsupported,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    enabled: bool,
    app_scope_id: Uuid,
    permission: Permission,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    En,
    Ru,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Sent,
    Duplicate,
    Suppressed,
    Denied,
    Stale,
}

fn main_only(window: &WebviewWindow) -> Result<(), &'static str> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("desktop_window_scope_denied")
    }
}
fn store(
    app: &AppHandle,
) -> Result<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>, &'static str> {
    app.store(STORE)
        .map_err(|_| "native_notification_storage_unavailable")
}
fn app_scope(app: &AppHandle) -> Result<Uuid, &'static str> {
    let store = store(app)?;
    if let Some(id) = store
        .get("app_scope_v1")
        .and_then(|value| value.as_str().and_then(|value| Uuid::parse_str(value).ok()))
    {
        if !id.is_nil() {
            return Ok(id);
        }
    }
    let id = Uuid::new_v4();
    store.set("app_scope_v1", serde_json::json!(id));
    store
        .save()
        .map_err(|_| "native_notification_storage_unavailable")?;
    Ok(id)
}
fn scope_current(app: &AppHandle, event: &Event) -> bool {
    if matches!(event.kind, Kind::RuntimeReady | Kind::DownloadReady) {
        return app_scope(app).is_ok_and(|id| id == event.scope_id);
    }
    app.state::<NotificationState>()
        .game_scope
        .lock()
        .is_ok_and(|scope| *scope == Some(event.scope_id))
}
fn app_focused(app: &AppHandle) -> bool {
    app.webview_windows()
        .values()
        .any(|window| window.is_focused().unwrap_or(true))
}
fn permission(app: &AppHandle) -> Permission {
    #[cfg(windows)]
    {
        use windows::{
            core::HSTRING,
            UI::Notifications::{NotificationSetting, ToastNotificationManager},
        };
        match ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            &app.config().identifier,
        ))
        .and_then(|notifier| notifier.Setting())
        {
            Ok(value) if value == NotificationSetting::Enabled => Permission::Granted,
            Ok(_) => Permission::Denied,
            Err(_) => Permission::Unknown,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Permission::Unsupported
    }
}
/// One monotonic series per encounter/actor or connection epoch; readiness uses
/// confirmed receipt UUIDs. Older restored/replayed counters cannot notify again.
fn series(event: &Event) -> Result<(String, u64), &'static str> {
    if event.scope_id.is_nil() || event.event_id.len() > 128 {
        return Err("native_notification_invalid");
    }
    let parts: Vec<_> = event.event_id.split(':').collect();
    let uuid = |value: &str| {
        Uuid::parse_str(value)
            .ok()
            .filter(|id| !id.is_nil())
            .ok_or("native_notification_invalid")
    };
    let count = |value: &str| {
        value
            .parse::<u64>()
            .ok()
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or("native_notification_invalid")
    };
    match event.kind {
        Kind::OwnTurn if event.target == Target::Chat && parts.len() == 3 => {
            let encounter = uuid(parts[0])?;
            let round = count(parts[1])?;
            let actor = uuid(parts[2])?;
            if round == 0 {
                return Err("native_notification_invalid");
            }
            Ok((
                format!("turn:{}:{encounter}:{actor}", event.scope_id),
                round,
            ))
        }
        Kind::ConnectionFailed if event.target == Target::Connection && parts.len() == 2 => {
            let epoch = count(parts[0])?;
            let cursor = count(parts[1])?;
            if epoch == 0 {
                return Err("native_notification_invalid");
            }
            Ok((
                format!("connection:{}:{epoch}", event.scope_id),
                cursor.saturating_add(1),
            ))
        }
        Kind::RuntimeReady | Kind::DownloadReady
            if event.target == Target::Downloads && parts.len() == 1 =>
        {
            let receipt = uuid(parts[0])?;
            let kind = if event.kind == Kind::RuntimeReady {
                "runtime"
            } else {
                "download"
            };
            Ok((format!("{kind}:{}:{receipt}", event.scope_id), 1))
        }
        _ => Err("native_notification_invalid"),
    }
}
fn consume(
    ledger: &mut BTreeMap<String, u64>,
    key: String,
    sequence: u64,
) -> Result<bool, &'static str> {
    if ledger.get(&key).is_some_and(|seen| *seen >= sequence) {
        return Ok(false);
    }
    if !ledger.contains_key(&key) && ledger.len() >= MAX_SERIES {
        return Err("native_notification_ledger_full");
    }
    ledger.insert(key, sequence);
    Ok(true)
}
#[cfg(windows)]
fn copy(kind: Kind, language: Language) -> Result<String, &'static str> {
    let source = match language {
        Language::En => include_str!("../../../src/locales/en/common.json"),
        Language::Ru => include_str!("../../../src/locales/ru/common.json"),
    };
    let key = match kind {
        Kind::OwnTurn => "notification_own_turn",
        Kind::RuntimeReady => "notification_runtime_ready",
        Kind::DownloadReady => "notification_download_ready",
        Kind::ConnectionFailed => "notification_connection_failed",
    };
    serde_json::from_str::<serde_json::Value>(source)
        .ok()
        .and_then(|catalog| {
            catalog
                .get(key)
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .ok_or("native_notification_catalog_invalid")
}
#[cfg(windows)]
fn activate(app: &AppHandle, event: &Event) {
    if let Some(window) = app.get_webview_window("main") {
        if window
            .show()
            .and_then(|()| window.unminimize())
            .and_then(|()| window.set_focus())
            .is_err()
        {
            log::error!("native notification action=focus_failed");
            return;
        }
        if scope_current(app, event) {
            app.state::<NotificationState>()
                .activations
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let _ = app.emit_to("main", "desktop-notification-activated", event);
        }
        log::info!("native notification action=activated");
    }
}
/// Debug QA diagnostics expose counts only, never another application's history.
#[tauri::command]
pub fn desktop_notification_qa_counts(
    window: WebviewWindow,
) -> Result<serde_json::Value, &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    if !cfg!(debug_assertions)
        || !app
            .config()
            .identifier
            .starts_with("com.kiselyovd.dungeonmasterai.qa.")
    {
        return Err("desktop_window_scope_denied");
    }
    #[cfg(windows)]
    {
        use windows::{core::HSTRING, UI::Notifications::ToastNotificationManager};
        let history = ToastNotificationManager::History()
            .and_then(|history| history.GetHistoryWithId(&HSTRING::from(&app.config().identifier)))
            .and_then(|history| history.Size())
            .map_err(|_| "native_notification_history_unavailable")?;
        let activations = app
            .state::<NotificationState>()
            .activations
            .load(std::sync::atomic::Ordering::SeqCst);
        let setting_error = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            &app.config().identifier,
        ))
        .and_then(|notifier| notifier.Setting())
        .err()
        .map(|error| error.code().0);
        Ok(
            serde_json::json!({"history":history,"activations":activations,"focused":app_focused(app),"settingError":setting_error}),
        )
    }
    #[cfg(not(windows))]
    {
        Err("native_notification_unavailable")
    }
}
/// Match the installed shortcut's AUMID for an isolated debug QA executable.
#[tauri::command]
pub fn desktop_notification_qa_shortcut(window: WebviewWindow) -> Result<(), &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    if !cfg!(debug_assertions)
        || app.config().identifier != "com.kiselyovd.dungeonmasterai.qa.notifications20261014"
    {
        return Err("desktop_window_scope_denied");
    }
    #[cfg(windows)]
    {
        use windows::{
            core::{Interface, HSTRING},
            Win32::{
                Foundation::PROPERTYKEY,
                System::Com::{
                    CoCreateInstance, IPersistFile, StructuredStorage::PROPVARIANT,
                    CLSCTX_INPROC_SERVER,
                },
                UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
            },
        };
        let path = app
            .path()
            .app_data_dir()
            .map_err(|_| "native_notification_shortcut_unavailable")?
            .parent()
            .ok_or("native_notification_shortcut_unavailable")?
            .join("Microsoft/Windows/Start Menu/Programs/Dungeon Master AI Notifications QA.lnk");
        if path.exists() {
            return Err("native_notification_shortcut_exists");
        }
        let exe =
            std::env::current_exe().map_err(|_| "native_notification_shortcut_unavailable")?;
        let result = (|| -> windows::core::Result<()> {
            unsafe {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                link.SetPath(&HSTRING::from(exe.to_string_lossy().as_ref()))?;
                let properties: IPropertyStore = link.cast()?;
                let key = PROPERTYKEY {
                    fmtid: windows::core::GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
                    pid: 5,
                };
                properties.SetValue(&key, &PROPVARIANT::from(app.config().identifier.as_str()))?;
                properties.Commit()?;
                let file: IPersistFile = link.cast()?;
                file.Save(&HSTRING::from(path.to_string_lossy().as_ref()), true)?;
                Ok(())
            }
        })();
        result.map_err(|_| "native_notification_shortcut_unavailable")
    }
    #[cfg(not(windows))]
    {
        Err("native_notification_unavailable")
    }
}
/// The Toolkit's identity-less app enrollment shows a suppressed toast once,
/// then removes it. QA verifies that this initializes the real OS setting.
#[tauri::command]
pub fn desktop_notification_qa_enroll(window: WebviewWindow) -> Result<(), &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    if !cfg!(debug_assertions)
        || app.config().identifier != "com.kiselyovd.dungeonmasterai.qa.notifications20261014"
    {
        return Err("desktop_window_scope_denied");
    }
    #[cfg(windows)]
    {
        use windows::{
            core::HSTRING,
            Data::Xml::Dom::XmlDocument,
            UI::Notifications::{ToastNotification, ToastNotificationManager},
        };
        let result = (|| -> windows::core::Result<()> {
            let xml = XmlDocument::new()?;
            xml.LoadXml(&HSTRING::from("<toast><visual><binding template='ToastGeneric'><text>Dungeon Master AI</text></binding></visual></toast>"))?;
            let toast = ToastNotification::CreateToastNotification(&xml)?;
            toast.SetSuppressPopup(true)?;
            toast.SetTag(&HSTRING::from("dmai-qa-enrollment"))?;
            toast.SetGroup(&HSTRING::from("dmai-qa"))?;
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                &app.config().identifier,
            ))?
            .Show(&toast)?;
            ToastNotificationManager::History()?.RemoveGroupedTagWithId(
                &HSTRING::from("dmai-qa-enrollment"),
                &HSTRING::from("dmai-qa"),
                &HSTRING::from(&app.config().identifier),
            )?;
            Ok(())
        })();
        result.map_err(|_| "native_notification_enrollment_failed")
    }
    #[cfg(not(windows))]
    {
        Err("native_notification_unavailable")
    }
}
#[tauri::command]
pub fn desktop_notification_preferences(
    window: WebviewWindow,
) -> Result<Preferences, &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    Ok(Preferences {
        enabled: store(app)?
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        app_scope_id: app_scope(app)?,
        permission: permission(app),
    })
}
#[tauri::command]
pub fn desktop_set_notifications(
    window: WebviewWindow,
    enabled: bool,
    language: Language,
) -> Result<Preferences, &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    #[cfg(windows)]
    if enabled {
        windows_delivery::enable(app)?;
    }
    let store = store(app)?;
    store.set("enabled", serde_json::json!(enabled));
    store.set(
        "language",
        serde_json::to_value(language).map_err(|_| "native_notification_invalid")?,
    );
    store
        .save()
        .map_err(|_| "native_notification_storage_unavailable")?;
    desktop_notification_preferences(window)
}
#[tauri::command]
pub fn desktop_notification_context(
    window: WebviewWindow,
    game_scope: Option<Uuid>,
    language: Option<Language>,
) -> Result<(), &'static str> {
    main_only(&window)?;
    if let Some(language) = language {
        let store = store(window.app_handle())?;
        let language = serde_json::to_value(language).map_err(|_| "native_notification_invalid")?;
        if store.get("language").as_ref() != Some(&language) {
            store.set("language", language);
            store
                .save()
                .map_err(|_| "native_notification_storage_unavailable")?;
        }
    }
    *window
        .state::<NotificationState>()
        .game_scope
        .lock()
        .map_err(|_| "native_notification_state_unavailable")? =
        game_scope.filter(|scope| !scope.is_nil());
    Ok(())
}
#[tauri::command]
pub fn desktop_offer_notification(
    window: WebviewWindow,
    event: Event,
    language: Language,
) -> Result<Outcome, &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    let state = app.state::<NotificationState>();
    let _lane = state
        .lane
        .lock()
        .map_err(|_| "native_notification_state_unavailable")?;
    let (key, sequence) = series(&event)?;
    if !scope_current(app, &event) {
        return Ok(Outcome::Stale);
    }
    let store = store(app)?;
    let mut ledger: BTreeMap<String, u64> = store
        .get("ledger_v1")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| "native_notification_ledger_invalid")?
        .unwrap_or_default();
    if !consume(&mut ledger, key, sequence)? {
        return Ok(Outcome::Duplicate);
    }
    store.set(
        "ledger_v1",
        serde_json::to_value(ledger).map_err(|_| "native_notification_ledger_invalid")?,
    );
    store
        .save()
        .map_err(|_| "native_notification_storage_unavailable")?;
    if app_focused(app)
        || !store
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    {
        return Ok(Outcome::Suppressed);
    }
    if permission(app) != Permission::Granted {
        return Ok(Outcome::Denied);
    }
    #[cfg(windows)]
    {
        let body = copy(event.kind, language)?;
        windows_delivery::show(app, &event, &body)?;
        log::info!("native notification action=submitted");
        Ok(Outcome::Sent)
    }
    #[cfg(not(windows))]
    {
        let _ = language;
        Ok(Outcome::Denied)
    }
}
pub fn resume(app: &AppHandle) {
    #[cfg(windows)]
    windows_delivery::resume(app);
    #[cfg(not(windows))]
    let _ = app;
}
pub fn shutdown() {
    #[cfg(windows)]
    windows_delivery::shutdown();
}
/// Only a completed native start reservation calls this; status polling cannot.
pub fn runtime_ready(app: &AppHandle) {
    let app = app.clone();
    let dispatcher = app.clone();
    let _ = dispatcher.run_on_main_thread(move || {
        let Some(window) = app.get_webview_window("main") else {
            return;
        };
        let Ok(scope_id) = app_scope(&app) else {
            return;
        };
        let language = match store(&app)
            .ok()
            .and_then(|store| store.get("language"))
            .and_then(|value| value.as_str().map(str::to_owned))
            .as_deref()
        {
            Some("ru") => Language::Ru,
            _ => Language::En,
        };
        let event = Event {
            kind: Kind::RuntimeReady,
            scope_id,
            event_id: Uuid::new_v4().to_string(),
            target: Target::Downloads,
        };
        if desktop_offer_notification(window, event, language).is_err() {
            log::warn!("native notification action=runtime_delivery_failed");
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> Event {
        Event {
            kind: Kind::OwnTurn,
            scope_id: Uuid::new_v4(),
            event_id: format!("{}:1:{}", Uuid::new_v4(), Uuid::new_v4()),
            target: Target::Chat,
        }
    }
    #[test]
    fn canonical_series_rejects_arbitrary_body_and_target() {
        let mut e = event();
        assert!(series(&e).is_ok());
        e.target = Target::Downloads;
        assert!(series(&e).is_err());
        e.target = Target::Chat;
        e.event_id = "private narration".into();
        assert!(series(&e).is_err());
    }
    #[test]
    fn ledger_retains_high_water_after_replay_and_serialization() {
        let (key, seq) = series(&event()).unwrap();
        let mut ledger = BTreeMap::new();
        assert_eq!(consume(&mut ledger, key.clone(), seq), Ok(true));
        let mut cold: BTreeMap<String, u64> =
            serde_json::from_value(serde_json::to_value(&ledger).unwrap()).unwrap();
        assert_eq!(consume(&mut cold, key.clone(), seq), Ok(false));
        assert_eq!(consume(&mut cold, key.clone(), seq + 1), Ok(true));
        assert_eq!(consume(&mut cold, key, seq), Ok(false));
    }
    #[test]
    fn ledger_capacity_fails_without_forgetting_replay_protection() {
        let mut ledger: BTreeMap<String, u64> =
            (0..MAX_SERIES).map(|id| (id.to_string(), 1)).collect();
        assert_eq!(
            consume(&mut ledger, "new".into(), 1),
            Err("native_notification_ledger_full")
        );
        assert_eq!(consume(&mut ledger, "0".into(), 1), Ok(false));
    }
}
