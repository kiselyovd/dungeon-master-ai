use app_llm::MessagePart;
use app_server::routes::chat::{ChatHttpRequest, HttpMessage};
use serde_json::json;

#[test]
fn user_string_content_normalises_to_single_text_part() {
    let payload = json!({
        "messages": [{"role": "user", "content": "hello"}],
    });
    let req: ChatHttpRequest = serde_json::from_value(payload).unwrap();
    assert_eq!(req.messages.len(), 1);
    let HttpMessage::User { parts } = &req.messages[0] else {
        panic!("expected User");
    };
    assert_eq!(parts.len(), 1);
    assert!(matches!(&parts[0], MessagePart::Text { text } if text == "hello"));
}

#[test]
fn user_parts_array_with_image_round_trips() {
    let payload = json!({
        "messages": [{
            "role": "user",
            "parts": [
                {"type": "text", "text": "see"},
                {"type": "image", "mime": "image/png", "data_b64": "aGk=", "name": null}
            ]
        }],
    });
    let req: ChatHttpRequest = serde_json::from_value(payload).unwrap();
    let HttpMessage::User { parts } = &req.messages[0] else {
        panic!("expected User");
    };
    assert_eq!(parts.len(), 2);
    assert!(matches!(&parts[0], MessagePart::Text { .. }));
    assert!(matches!(&parts[1], MessagePart::Image { mime, .. } if mime == "image/png"));
}

#[test]
fn system_keeps_string_content() {
    let payload = json!({
        "messages": [
            {"role": "system", "content": "you are dm"},
            {"role": "user", "content": "hi"}
        ],
    });
    let req: ChatHttpRequest = serde_json::from_value(payload).unwrap();
    assert!(matches!(&req.messages[0], HttpMessage::System { content } if content == "you are dm"));
}

#[test]
fn unknown_role_rejected() {
    let payload = json!({
        "messages": [{"role": "wizard", "content": "x"}],
    });
    let res: Result<ChatHttpRequest, _> = serde_json::from_value(payload);
    assert!(res.is_err());
}
