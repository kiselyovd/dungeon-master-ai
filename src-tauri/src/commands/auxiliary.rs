//! Detached views receive only the main window's current, short-lived projection.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use uuid::Uuid;

const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
const LEASE: Duration = Duration::from_secs(15);
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuxiliaryKind {
    Hero,
    Journal,
}
impl AuxiliaryKind {
    fn label(self) -> &'static str {
        match self {
            Self::Hero => "hero",
            Self::Journal => "journal",
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuxiliaryFrame {
    schema_version: u8,
    sequence: u64,
    scope_id: String,
    campaign_id: Option<Uuid>,
    actor_id: Option<Uuid>,
    member_id: Option<Uuid>,
    revision: u64,
    language: String,
    ready: bool,
    hero: Option<Value>,
    journal: Vec<Value>,
}
#[derive(Default)]
pub struct AuxiliaryState {
    current: Mutex<Option<(AuxiliaryFrame, Instant)>>,
    windows: tokio::sync::Mutex<()>,
}
fn main_only(window: &WebviewWindow) -> Result<(), &'static str> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("desktop_window_scope_denied")
    }
}
fn validate(frame: &AuxiliaryFrame) -> Result<(), &'static str> {
    if frame.schema_version != 1
        || frame.scope_id.len() > 256
        || !["en", "ru"].contains(&frame.language.as_str())
        || frame.journal.len() > 2000
        || serde_json::to_vec(frame)
            .map_err(|_| "desktop_aux_invalid")?
            .len()
            > MAX_FRAME_BYTES
    {
        return Err("desktop_aux_invalid");
    }
    if !frame.ready && (frame.hero.is_some() || !frame.journal.is_empty()) {
        return Err("desktop_aux_invalid");
    }
    if frame.ready && frame.campaign_id.is_none() {
        return Err("desktop_aux_invalid");
    }
    if let Some(hero) = &frame.hero {
        let object = hero.as_object().ok_or("desktop_aux_invalid")?;
        let allowed = [
            "defense",
            "baseSpeedFt",
            "characterId",
            "backstory",
            "ideals",
            "bonds",
            "flaws",
            "personalityFlags",
            "spells",
            "heroClass",
            "name",
            "race",
            "subclass",
            "background",
            "alignment",
            "level",
            "experience",
            "experienceNext",
            "hp",
            "hpMax",
            "ac",
            "initiative",
            "speedFt",
            "proficiencyBonus",
            "abilities",
            "savingThrowProfs",
            "skillProfs",
            "inventory",
            "portraitUrl",
            "portraitAssetId",
            "portraitMediaRef",
        ];
        if object.keys().any(|key| !allowed.contains(&key.as_str()))
            || object
                .get("portraitUrl")
                .is_some_and(|value| !value.is_null())
            || object
                .get("characterId")
                .and_then(Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok())
                != frame.actor_id
        {
            return Err("desktop_aux_invalid");
        }
    }
    for entry in &frame.journal {
        let entry = entry.as_object().ok_or("desktop_aux_invalid")?;
        if entry.keys().any(|key| {
            !["id", "campaign_id", "chapter", "entry_html", "created_at"].contains(&key.as_str())
        }) || entry
            .get("campaign_id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            != frame.campaign_id
        {
            return Err("desktop_aux_invalid");
        }
    }
    Ok(())
}
#[tauri::command]
pub fn desktop_aux_publish(
    window: WebviewWindow,
    frame: AuxiliaryFrame,
) -> Result<(), &'static str> {
    main_only(&window)?;
    let state = window.state::<AuxiliaryState>();
    let mut current = state
        .current
        .lock()
        .map_err(|_| "desktop_aux_unavailable")?;
    if current
        .as_ref()
        .is_some_and(|(old, _)| frame.sequence <= old.sequence)
    {
        return Ok(());
    }
    if let Err(code) = validate(&frame) {
        *current = None;
        return Err(code);
    }
    if frame.ready
        && !super::rooms::auxiliary_scope_allowed(
            window.app_handle(),
            frame.campaign_id,
            frame.member_id,
        )
    {
        *current = None;
        return Err("desktop_aux_scope_revoked");
    }
    *current = Some((frame, Instant::now()));
    Ok(())
}
fn project(frame: &AuxiliaryFrame, kind: AuxiliaryKind) -> Value {
    json!({"schemaVersion":1,"sequence":frame.sequence,"scopeId":frame.scope_id,"campaignId":frame.campaign_id,"actorId":frame.actor_id,"revision":frame.revision,"language":frame.language,"ready":frame.ready,
        "hero": if matches!(kind,AuxiliaryKind::Hero) {frame.hero.clone()} else {None},
        "journal": if matches!(kind,AuxiliaryKind::Journal) {frame.journal.clone()} else {vec![]}})
}
#[tauri::command]
pub fn desktop_aux_heartbeat(window: WebviewWindow, sequence: u64) -> Result<(), &'static str> {
    main_only(&window)?;
    let state = window.state::<AuxiliaryState>();
    let mut current = state
        .current
        .lock()
        .map_err(|_| "desktop_aux_unavailable")?;
    if let Some((frame, time)) = current
        .as_mut()
        .filter(|(frame, _)| frame.sequence == sequence)
    {
        if !frame.ready
            || super::rooms::auxiliary_scope_allowed(
                window.app_handle(),
                frame.campaign_id,
                frame.member_id,
            )
        {
            *time = Instant::now();
        }
    }
    Ok(())
}
#[tauri::command]
pub fn desktop_aux_read(
    window: WebviewWindow,
    after_sequence: Option<u64>,
) -> Result<Option<Value>, &'static str> {
    let kind = match window.label() {
        "hero" => AuxiliaryKind::Hero,
        "journal" => AuxiliaryKind::Journal,
        _ => return Err("desktop_window_scope_denied"),
    };
    let state = window.state::<AuxiliaryState>();
    let current = state
        .current
        .lock()
        .map_err(|_| "desktop_aux_unavailable")?;
    Ok(current
        .as_ref()
        .filter(|(frame, time)| {
            time.elapsed() < LEASE
                && (!frame.ready
                    || super::rooms::auxiliary_scope_allowed(
                        window.app_handle(),
                        frame.campaign_id,
                        frame.member_id,
                    ))
        })
        .map(|(frame, _)| {
            if after_sequence == Some(frame.sequence) {
                json!({"unchangedSequence":frame.sequence})
            } else {
                project(frame, kind)
            }
        }))
}
#[tauri::command]
pub async fn desktop_aux_open(window: WebviewWindow, kind: AuxiliaryKind) -> Result<(), String> {
    main_only(&window).map_err(str::to_owned)?;
    let app = window.app_handle();
    let state = app.state::<AuxiliaryState>();
    let _window_lane = state.windows.lock().await;
    if let Some(existing) = app.get_webview_window(kind.label()) {
        existing
            .show()
            .and_then(|()| existing.unminimize())
            .and_then(|()| existing.set_focus())
            .map_err(|_| "desktop_aux_unavailable".to_owned())?;
        return Ok(());
    }
    let profile = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "desktop_aux_unavailable")?
        .join(format!("auxiliary-{}", kind.label()));
    let detached=WebviewWindowBuilder::new(app,kind.label(),WebviewUrl::App("auxiliary.html".into()))
        .title("Dungeon Master AI")
        .inner_size(960.0,720.0).min_inner_size(520.0,400.0)
        .data_directory(profile)
        .incognito(true)
        .on_navigation(|url|url.scheme()=="tauri" || url.host_str()==Some("tauri.localhost") || cfg!(dev) && url.host_str()==Some("127.0.0.1"))
        .on_web_resource_request(|_,response|{
            response.headers_mut().insert("Content-Security-Policy", "default-src 'self'; base-uri 'none'; object-src 'none'; frame-src 'none'; form-action 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; img-src 'self' data: blob:; connect-src ipc: http://ipc.localhost; worker-src 'none'".parse().expect("static CSP"));
        })
        .build().map_err(|_|"desktop_aux_unavailable".to_owned())?;
    super::desktop::restore_auxiliary(&detached.as_ref().window());
    log::info!("desktop auxiliary window={} action=open", kind.label());
    Ok(())
}
#[tauri::command]
pub fn desktop_aux_focus_main(window: WebviewWindow) -> Result<(), &'static str> {
    if !["hero", "journal"].contains(&window.label()) {
        return Err("desktop_window_scope_denied");
    }
    let main = window
        .app_handle()
        .get_webview_window("main")
        .ok_or("desktop_aux_unavailable")?;
    main.show()
        .and_then(|()| main.unminimize())
        .and_then(|()| main.set_focus())
        .map_err(|_| "desktop_aux_unavailable")
}
#[tauri::command]
pub fn desktop_aux_close(window: WebviewWindow) -> Result<(), &'static str> {
    if !["hero", "journal"].contains(&window.label()) {
        return Err("desktop_window_scope_denied");
    }
    window.close().map_err(|_| "desktop_aux_unavailable")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> AuxiliaryFrame {
        let campaign = Uuid::new_v4();
        let actor = Uuid::new_v4();
        AuxiliaryFrame {
            schema_version: 1,
            sequence: 1,
            scope_id: "owned".into(),
            campaign_id: Some(campaign),
            actor_id: Some(actor),
            member_id: None,
            revision: 2,
            language: "ru".into(),
            ready: true,
            hero: Some(json!({"characterId":actor,"name":"Mira","portraitUrl":null})),
            journal: vec![
                json!({"id":"entry","campaign_id":campaign,"chapter":null,"entry_html":"Published","created_at":"2026-10-14"}),
            ],
        }
    }
    #[test]
    fn each_view_receives_only_its_projection() {
        let frame = frame();
        validate(&frame).unwrap();
        let journal = project(&frame, AuxiliaryKind::Journal);
        assert!(journal["hero"].is_null());
        let hero = project(&frame, AuxiliaryKind::Hero);
        assert!(hero["journal"].as_array().unwrap().is_empty());
    }
    #[test]
    fn wrong_actor_private_fields_external_portrait_and_cross_campaign_journal_fail_closed() {
        let mut value = frame();
        value.hero.as_mut().unwrap()["characterId"] = json!(Uuid::new_v4());
        assert!(validate(&value).is_err());
        let mut value = frame();
        value.hero.as_mut().unwrap()["provider_token"] = json!("fixture");
        assert!(validate(&value).is_err());
        let mut value = frame();
        value.hero.as_mut().unwrap()["portraitUrl"] = json!("http://127.0.0.1/private");
        assert!(validate(&value).is_err());
        let mut value = frame();
        value.journal[0]["campaign_id"] = json!(Uuid::new_v4());
        assert!(validate(&value).is_err());
        let mut value = frame();
        value.ready = false;
        assert!(validate(&value).is_err());
    }
}
