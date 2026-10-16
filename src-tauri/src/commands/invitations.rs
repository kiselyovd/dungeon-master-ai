//! Invitations are a bounded join preview, never an implicit connection.
use std::{collections::VecDeque, net::SocketAddr, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_deep_link::DeepLinkExt;
use uuid::Uuid;

const INVALID: &str = "native_invitation_invalid";

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvitationPreview {
    pub address: SocketAddr,
    pub room_id: Uuid,
    pub invitation: String,
}

#[derive(Clone, serde::Serialize)]
pub struct InvitationEvent {
    pub event_id: Uuid,
    pub preview: InvitationPreview,
}

#[derive(Default)]
pub struct InvitationInbox {
    pending: Mutex<VecDeque<InvitationEvent>>,
}

pub fn scheme(app: &AppHandle) -> String {
    let identifier = &app.config().identifier;
    if identifier.starts_with("com.kiselyovd.dungeonmasterai.qa.") {
        format!(
            "dmai-qa-{}",
            identifier.rsplit('.').next().unwrap_or("isolated")
        )
    } else {
        "dmai".into()
    }
}

fn validate(preview: &InvitationPreview) -> Result<(), &'static str> {
    let ip = preview.address.ip();
    let invalid_ipv4 = match ip {
        std::net::IpAddr::V4(ip) => Some(ip),
        std::net::IpAddr::V6(ip) => ip.to_ipv4(),
    }
    .is_some_and(|ip| ip.is_unspecified() || ip.octets()[0] >= 224);
    if preview.address.port() == 0
        || ip.is_unspecified()
        || ip.is_multicast()
        || invalid_ipv4
        || preview.room_id.is_nil()
        || preview.invitation.len() != 64
        || !preview
            .invitation
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(INVALID);
    }
    Ok(())
}

pub fn parse(raw: &str, allowed_scheme: &str) -> Result<InvitationPreview, &'static str> {
    if raw.len() > 1024 || raw.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(INVALID);
    }
    let uri = reqwest::Url::parse(raw).map_err(|_| INVALID)?;
    if raw.split('?').next() != Some(format!("{}://join/v1", uri.scheme()).as_str()) {
        return Err(INVALID);
    }
    if !["dmai", allowed_scheme].contains(&uri.scheme())
        || uri.host_str() != Some("join")
        || uri.path() != "/v1"
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.port().is_some()
        || uri.fragment().is_some()
    {
        return Err(INVALID);
    }
    let mut address = None;
    let mut room_id = None;
    let mut invitation = None;
    for (key, value) in uri.query_pairs() {
        match key.as_ref() {
            "address" if address.is_none() => {
                address = Some(value.parse::<SocketAddr>().map_err(|_| INVALID)?)
            }
            "room" if room_id.is_none() => {
                room_id = Some(Uuid::parse_str(&value).map_err(|_| INVALID)?)
            }
            "invitation" if invitation.is_none() => invitation = Some(value.into_owned()),
            _ => return Err(INVALID),
        }
    }
    let preview = InvitationPreview {
        address: address.ok_or(INVALID)?,
        room_id: room_id.ok_or(INVALID)?,
        invitation: invitation.ok_or(INVALID)?,
    };
    validate(&preview)?;
    Ok(preview)
}

pub fn queue(app: &AppHandle, raw: &str) -> Result<(), &'static str> {
    let preview = parse(raw, &scheme(app))?;
    let inbox = app.state::<InvitationInbox>();
    let mut pending = inbox
        .pending
        .lock()
        .map_err(|_| "native_invitation_inbox_unavailable")?;
    if pending.iter().any(|event| event.preview == preview) {
        return Ok(());
    }
    if pending.len() >= 4 {
        return Err("native_invitation_inbox_full");
    }
    let event_id = Uuid::new_v4();
    pending.push_back(InvitationEvent { event_id, preview });
    drop(pending);
    // Only the event ID crosses the event bus. The main window claims the preview.
    let _ = app.emit_to("main", "desktop-invitation-ready", event_id);
    log::info!("native invitation result=preview_queued event_id={event_id}");
    Ok(())
}

fn main_only(window: &WebviewWindow) -> Result<(), &'static str> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("desktop_window_scope_denied")
    }
}

#[tauri::command]
pub fn desktop_parse_invitation(
    window: WebviewWindow,
    raw: String,
) -> Result<InvitationPreview, &'static str> {
    main_only(&window)?;
    parse(&raw, &scheme(window.app_handle()))
}

#[tauri::command]
pub fn desktop_take_invitation(
    window: WebviewWindow,
) -> Result<Option<InvitationEvent>, &'static str> {
    main_only(&window)?;
    window
        .state::<InvitationInbox>()
        .pending
        .lock()
        .map_err(|_| "native_invitation_inbox_unavailable")
        .map(|mut pending| pending.pop_front())
}

#[tauri::command]
pub fn desktop_create_invitation(
    window: WebviewWindow,
    preview: InvitationPreview,
) -> Result<String, &'static str> {
    main_only(&window)?;
    validate(&preview)?;
    let mut uri = reqwest::Url::parse("dmai://join/v1").map_err(|_| INVALID)?;
    uri.query_pairs_mut()
        .append_pair("address", &preview.address.to_string())
        .append_pair("room", &preview.room_id.to_string())
        .append_pair("invitation", &preview.invitation);
    Ok(uri.into())
}

#[tauri::command]
pub fn desktop_register_invitation_protocol(window: WebviewWindow) -> Result<(), &'static str> {
    main_only(&window)?;
    let app = window.app_handle();
    app.deep_link()
        .register(scheme(app))
        .map_err(|_| "native_invitation_protocol_unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn uri(address: &str) -> String {
        format!("dmai://join/v1?address={address}&room=00000000-0000-4000-8000-000000000001&invitation={}", "a".repeat(64))
    }
    #[test]
    fn valid_links_provide_only_existing_join_fields() {
        for address in [
            "192.168.1.20:4123",
            "127.0.0.1:4123",
            "%5B%3A%3A1%5D%3A4123",
        ] {
            let preview = parse(&uri(address), "dmai").unwrap();
            assert_eq!(preview.address.port(), 4123);
            assert!(!preview.room_id.is_nil());
            assert_eq!(preview.invitation.len(), 64);
        }
    }
    #[test]
    fn hostile_or_ambiguous_links_fail_without_payload_in_the_error() {
        let valid = uri("127.0.0.1:4123");
        for raw in [
            valid.replace("dmai:", "https:"),
            valid.replace("/v1", "/v2"),
            format!("{valid}#private"),
            format!("{valid}&capability={}", "b".repeat(64)),
            format!("{valid}&room=00000000-0000-4000-8000-000000000002"),
            valid.replace("join/", "user@join/"),
            valid.replace("join/", "join:80/"),
            uri("0.0.0.0:4123"),
            uri("224.0.0.1:4123"),
            uri("255.255.255.255:4123"),
            uri("127.0.0.1:0"),
            uri("localhost:4123"),
            uri("http%3A%2F%2F127.0.0.1%3A4123"),
            valid.replace("invitation=", "resume_proof="),
            valid.replace(&"a".repeat(64), "short"),
            format!("{valid}\n"),
            "a".repeat(1025),
        ] {
            assert!(matches!(parse(&raw, "dmai"), Err(INVALID)));
        }
    }
    #[test]
    fn qa_protocol_is_accepted_only_for_its_own_identity() {
        let qa = uri("127.0.0.1:4123").replacen("dmai:", "dmai-qa-isolated:", 1);
        assert!(parse(&qa, "dmai-qa-isolated").is_ok());
        assert!(parse(&qa, "dmai").is_err());
        assert!(parse(&qa, "dmai-qa-another").is_err());
    }
}
