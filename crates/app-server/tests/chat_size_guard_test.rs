use app_llm::{ChatMessage, MessagePart};
use app_server::routes::chat::enforce_size_guards;
use base64::Engine;

#[test]
fn rejects_image_over_5_mb() {
    let raw = vec![0u8; 5 * 1024 * 1024 + 1];
    let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
    let messages = vec![ChatMessage::User {
        parts: vec![MessagePart::Image {
            mime: "image/png".into(),
            data_b64: b64,
            name: None,
        }],
    }];
    let err = enforce_size_guards(&messages).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("5 MB"), "expected 5 MB in error, got: {msg}");
}

#[test]
fn rejects_more_than_four_images() {
    let img = MessagePart::Image {
        mime: "image/png".into(),
        data_b64: "aGk=".into(),
        name: None,
    };
    let messages = vec![ChatMessage::User {
        parts: vec![
            img.clone(),
            img.clone(),
            img.clone(),
            img.clone(),
            img.clone(),
        ],
    }];
    let err = enforce_size_guards(&messages).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("4 image"),
        "expected '4 image' in error, got: {msg}"
    );
}

#[test]
fn accepts_under_limits() {
    let messages = vec![ChatMessage::User {
        parts: vec![
            MessagePart::Text { text: "hi".into() },
            MessagePart::Image {
                mime: "image/png".into(),
                data_b64: "aGk=".into(),
                name: None,
            },
        ],
    }];
    enforce_size_guards(&messages).unwrap();
}

#[test]
fn rejects_invalid_base64() {
    let messages = vec![ChatMessage::User {
        parts: vec![MessagePart::Image {
            mime: "image/png".into(),
            data_b64: "this is not base64!!!".into(),
            name: None,
        }],
    }];
    let err = enforce_size_guards(&messages).unwrap_err();
    assert!(err.to_string().contains("invalid base64"));
}

#[test]
fn ignores_non_user_messages() {
    let messages = vec![
        ChatMessage::System {
            content: "x".repeat(10_000_000),
        },
        ChatMessage::Assistant {
            content: "y".repeat(10_000_000),
        },
    ];
    enforce_size_guards(&messages).unwrap();
}
