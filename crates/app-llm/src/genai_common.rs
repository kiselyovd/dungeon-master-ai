//! Shared genai helpers used by `OpenAICompatProvider` and
//! `MistralrsLocalProvider`. Keeps message conversion and the streaming
//! `ToolCallChunk -> ChatChunk` pump in one place so the providers do not
//! drift. (Native Anthropic was removed in M11 Batch D.5; cloud chat now runs
//! exclusively through the OpenAI-compatible provider.)

use futures::stream::StreamExt;
use genai::chat::{
    ChatMessage as GMsg, ChatRequest as GReq, ChatStream, ChatStreamEvent, Tool as GTool,
    ToolCall as GToolCall, ToolResponse as GToolResponse,
};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::warn;

use crate::provider::{ChatChunk, ChatMessage, FinishReason, LlmError, MessagePart, Tool};

/// Translate our provider-agnostic `ChatMessage`/`Tool` shape into a genai
/// `ChatRequest`. Identical for every genai-backed provider, so it lives here.
pub(crate) fn convert_messages(messages: Vec<ChatMessage>, tools: Vec<Tool>) -> GReq {
    let mut g_req = GReq::default();
    for tool in tools {
        let g_tool = GTool::new(tool.name)
            .with_description(tool.description)
            .with_schema(tool.parameters);
        g_req = g_req.append_tool(g_tool);
    }
    for m in messages {
        match m {
            ChatMessage::System { content } => g_req = g_req.with_system(content),
            ChatMessage::User { parts } => {
                use genai::chat::{ContentPart, MessageContent};
                let mut content = MessageContent::default();
                for part in parts {
                    match part {
                        MessagePart::Text { text } => {
                            content.push(ContentPart::Text(text));
                        }
                        MessagePart::Image {
                            mime,
                            data_b64,
                            name,
                        } => {
                            content.push(ContentPart::from_binary_base64(mime, data_b64, name));
                        }
                    }
                }
                g_req = g_req.append_message(GMsg::user(content));
            }
            ChatMessage::Assistant { content } => {
                g_req = g_req.append_message(GMsg::assistant(content));
            }
            ChatMessage::AssistantWithToolCalls {
                content,
                tool_calls,
            } => {
                if let Some(text) = content {
                    g_req = g_req.append_message(GMsg::assistant(text));
                }
                let g_calls: Vec<GToolCall> = tool_calls
                    .into_iter()
                    .map(|tc| GToolCall {
                        call_id: tc.id,
                        fn_name: tc.name,
                        fn_arguments: tc.args,
                        thought_signatures: None,
                    })
                    .collect();
                if !g_calls.is_empty() {
                    g_req = g_req.append_message(GMsg::from(g_calls));
                }
            }
            ChatMessage::ToolResult(tr) => {
                // TODO(phase-c): tr.is_error is dropped - genai's ToolResponse has no error variant. Encode error semantics into tr.content instead (e.g. JSON {"error": "..."}).
                let response = GToolResponse::new(tr.tool_call_id, tr.content);
                g_req = g_req.append_message(GMsg::from(response));
            }
        }
    }
    g_req
}

/// Leading marker mistralrs-cli leaks into `delta.content` when a local model
/// (Gemma 4) emits a tool call. The binary ALSO surfaces a clean structured
/// `delta.tool_calls` in the same chunk, but genai (0.6.x) does not yield a
/// `ToolCallChunk` when content and tool_calls share a chunk - so we recover
/// the call from the leaked text instead. See `parse_leaked_tool_calls`.
const LEAKED_TOOL_MARKER: &str = "<|tool_call>";

/// Parse mistralrs/Gemma's leaked tool-call text into `(fn_name, args_json)`
/// pairs. The raw text looks like:
///   `<|tool_call>call:start_combat{initiative_entries:[{ac:16,id:<|"|>hero<|"|>,...}]}`
/// i.e. unquoted object keys, with string values wrapped in the `<|"|>` quote
/// token. We normalise that back into valid JSON. Returns an empty vec when no
/// well-formed call is found (caller then keeps the text as-is).
fn parse_leaked_tool_calls(content: &str) -> Vec<(String, String)> {
    const CALL: &str = "<|tool_call>call:";
    let mut out = Vec::new();
    let mut rest = content;
    while let Some(idx) = rest.find(CALL) {
        let after = &rest[idx + CALL.len()..];
        let Some(brace) = after.find('{') else { break };
        let name = after[..brace].trim().to_string();
        let args_region = &after[brace..];
        let Some(end) = balanced_brace_end(args_region) else {
            break;
        };
        if let Some(json) = normalise_leaked_args(&args_region[..=end]) {
            if !name.is_empty() {
                out.push((name, json));
            }
        }
        rest = &args_region[end + 1..];
    }
    out
}

/// Byte index of the `}` that closes the object opened by the leading `{`.
/// Brace counting is safe on the raw text: the only strings present are wrapped
/// in `<|"|>` and never contain `{`/`}`.
fn balanced_brace_end(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, b) in s.bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Turn the `<|"|>`-quoted, bare-key pseudo-JSON into real JSON. Returns `None`
/// if the result does not parse (caller falls back to emitting the raw text).
fn normalise_leaked_args(raw: &str) -> Option<String> {
    let replaced = raw.replace("<|\"|>", "\"");
    let quoted = quote_bare_keys(&replaced);
    serde_json::from_str::<serde_json::Value>(&quoted).ok()?;
    Some(quoted)
}

/// Quote bare object keys (an identifier right after `{`, `[`, or `,` and
/// followed by `:`). Char-based so UTF-8 string values (e.g. Cyrillic names)
/// survive; content already inside a `"..."` string is copied verbatim.
fn quote_bare_keys(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut chars = s.chars().peekable();
    let mut in_str = false;
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push('"');
            }
            '{' | '[' | ',' => {
                out.push(c);
                while let Some(&w) = chars.peek() {
                    if w.is_whitespace() {
                        out.push(w);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let mut ident = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !ident.is_empty() && chars.peek() == Some(&':') {
                    out.push('"');
                    out.push_str(&ident);
                    out.push('"');
                } else {
                    out.push_str(&ident);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Three-way state for the leaked-tool-call content buffer (see
/// `pump_genai_stream`). `Undecided` holds content until we can tell whether it
/// is a leaked tool call (`Tool`, suppress + synthesize) or narration (`Text`,
/// stream it through).
enum ContentMode {
    Undecided,
    Text,
    Tool,
}

/// Drain a genai chat stream, translating each event into our `ChatChunk`
/// shape and forwarding it on `tx`. Bails out as soon as the receiver is
/// dropped. The genai stream is consumed by value.
pub(crate) async fn pump_genai_stream(
    mut g_stream: ChatStream,
    tx: Sender<Result<ChatChunk, LlmError>>,
) {
    let mut args_seen: HashMap<String, String> = HashMap::new();
    let mut content_mode = ContentMode::Undecided;
    let mut pending = String::new();
    let mut tool_text = String::new();

    while let Some(event) = g_stream.next().await {
        match event {
            Ok(ChatStreamEvent::Chunk(c)) => match content_mode {
                ContentMode::Tool => tool_text.push_str(&c.content),
                ContentMode::Text => {
                    if tx
                        .send(Ok(ChatChunk::TextDelta { text: c.content }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                ContentMode::Undecided => {
                    pending.push_str(&c.content);
                    if let Some(pos) = pending.find(LEAKED_TOOL_MARKER) {
                        content_mode = ContentMode::Tool;
                        tool_text = pending[pos..].to_string();
                        pending.clear();
                    } else if !LEAKED_TOOL_MARKER.starts_with(pending.trim_start()) {
                        // Diverged from the marker prefix: this is narration.
                        content_mode = ContentMode::Text;
                        let text = std::mem::take(&mut pending);
                        if !text.is_empty()
                            && tx.send(Ok(ChatChunk::TextDelta { text })).await.is_err()
                        {
                            return;
                        }
                    }
                    // else: still a possible marker prefix - keep buffering.
                }
            },
            Ok(ChatStreamEvent::ReasoningChunk(c)) => {
                if tx
                    .send(Ok(ChatChunk::ThinkingDelta { text: c.content }))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Ok(ChatStreamEvent::ThoughtSignatureChunk(_)) => continue,
            Ok(ChatStreamEvent::ToolCallChunk(tc)) => {
                let call_id = tc.tool_call.call_id.clone();
                let name = tc.tool_call.fn_name.clone();
                let current_args = match tc.tool_call.fn_arguments {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                let prev_args = args_seen.get(&call_id).cloned().unwrap_or_default();

                if !args_seen.contains_key(&call_id)
                    && tx
                        .send(Ok(ChatChunk::ToolCallStart {
                            id: call_id.clone(),
                            name,
                        }))
                        .await
                        .is_err()
                {
                    return;
                }

                if current_args != prev_args {
                    let delta = match current_args.strip_prefix(&prev_args) {
                        Some(suffix) => suffix.to_string(),
                        None => {
                            warn!(
                                call_id = %call_id,
                                prev_len = prev_args.len(),
                                current_len = current_args.len(),
                                "genai cumulative-args invariant broken, sending full string as delta"
                            );
                            current_args.clone()
                        }
                    };
                    if !delta.is_empty()
                        && tx
                            .send(Ok(ChatChunk::ToolCallArgsDelta {
                                id: call_id.clone(),
                                args_fragment: delta,
                            }))
                            .await
                            .is_err()
                    {
                        return;
                    }
                }

                args_seen.insert(call_id, current_args);
            }
            Ok(ChatStreamEvent::Start) => continue,
            Ok(ChatStreamEvent::End(_end)) => {
                // Undecided content that never became a tool call is narration.
                if matches!(content_mode, ContentMode::Undecided) && !pending.is_empty() {
                    let text = std::mem::take(&mut pending);
                    let _ = tx.send(Ok(ChatChunk::TextDelta { text })).await;
                }
                // Recover any leaked tool call(s) from the suppressed content.
                let mut synthesized = false;
                if matches!(content_mode, ContentMode::Tool) {
                    let calls = parse_leaked_tool_calls(&tool_text);
                    for (i, (name, args)) in calls.into_iter().enumerate() {
                        let id = format!("leaked-{i}");
                        if tx
                            .send(Ok(ChatChunk::ToolCallStart {
                                id: id.clone(),
                                name,
                            }))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        if tx
                            .send(Ok(ChatChunk::ToolCallArgsDelta {
                                id: id.clone(),
                                args_fragment: args,
                            }))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        if tx.send(Ok(ChatChunk::ToolCallDone { id })).await.is_err() {
                            return;
                        }
                        synthesized = true;
                    }
                    // Marker seen but nothing parsed: do not swallow the text.
                    if !synthesized && !tool_text.is_empty() {
                        let text = std::mem::take(&mut tool_text);
                        let _ = tx.send(Ok(ChatChunk::TextDelta { text })).await;
                    }
                }
                let had_tool_calls = !args_seen.is_empty() || synthesized;
                for id in std::mem::take(&mut args_seen).into_keys() {
                    if tx.send(Ok(ChatChunk::ToolCallDone { id })).await.is_err() {
                        return;
                    }
                }
                let reason = if had_tool_calls {
                    FinishReason::ToolUse
                } else {
                    FinishReason::Stop
                };
                let _ = tx.send(Ok(ChatChunk::Done { reason })).await;
                return;
            }
            Err(e) => {
                warn!("genai stream error: {e}");
                let _ = tx.send(Err(classify_genai_error(e.to_string()))).await;
                return;
            }
        }
    }
}

/// Build a `genai::chat::ChatOptions` from an optional `ReasoningSpec`.
/// Returns `None` when reasoning is not requested (providers can pass `None`
/// to genai and get the same behaviour as before).
pub(crate) fn build_chat_options(
    reasoning: Option<crate::provider::ReasoningSpec>,
) -> Option<genai::chat::ChatOptions> {
    let spec = reasoning?;
    Some(
        genai::chat::ChatOptions::default()
            .with_reasoning_effort(spec.to_genai_effort())
            .with_capture_reasoning_content(true),
    )
}

/// Map a genai error string into our `LlmError` taxonomy. Kept in this module
/// so providers do not need to duplicate it.
pub(crate) fn classify_genai_error(msg: String) -> LlmError {
    let lower = msg.to_lowercase();
    if lower.contains("rate") {
        LlmError::RateLimit
    } else if lower.contains("auth") || lower.contains("api key") || lower.contains("401") {
        LlmError::AuthFailure
    } else if lower.contains("network") || lower.contains("connection") || lower.contains("timeout")
    {
        LlmError::Network(msg)
    } else {
        LlmError::Provider(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_with_image_part_emits_binary_in_content() {
        let messages = vec![ChatMessage::User {
            parts: vec![
                MessagePart::Text {
                    text: "look".into(),
                },
                MessagePart::Image {
                    mime: "image/png".into(),
                    data_b64: "aGVsbG8=".into(),
                    name: None,
                },
            ],
        }];
        let g_req = convert_messages(messages, vec![]);
        assert_eq!(g_req.messages.len(), 1);
        let content = &g_req.messages[0].content;
        assert!(content.contains_text(), "must keep the text part");
        assert!(content.contains_binary(), "must carry an image part");
        let bins = content.binaries();
        assert_eq!(bins.len(), 1);
        assert!(bins[0].content_type.starts_with("image/"));
    }

    #[test]
    fn user_text_only_does_not_produce_binary() {
        let messages = vec![ChatMessage::user_text("hi")];
        let g_req = convert_messages(messages, vec![]);
        assert_eq!(g_req.messages.len(), 1);
        let content = &g_req.messages[0].content;
        assert_eq!(content.first_text(), Some("hi"));
        assert!(!content.contains_binary());
    }

    #[test]
    fn classify_rate_limit() {
        assert!(matches!(
            classify_genai_error("rate limit hit".into()),
            LlmError::RateLimit
        ));
    }

    #[test]
    fn build_chat_options_none_when_no_reasoning() {
        assert!(build_chat_options(None).is_none());
    }

    #[test]
    fn parse_leaked_tool_calls_recovers_clean_json() {
        // Exact shape mistralrs-cli leaks into delta.content for Gemma 4 tool
        // calls (captured live): unquoted keys, `<|"|>`-wrapped string values.
        let raw = concat!(
            "<|tool_call>call:start_combat{initiative_entries:[",
            "{ac:16,dex_mod:1,hp:12,id:<|\"|>hero<|\"|>,max_hp:12,name:<|\"|>Hero the Fighter<|\"|>,roll:25},",
            "{ac:15,dex_mod:2,hp:7,id:<|\"|>goblin_a<|\"|>,max_hp:7,name:<|\"|>Goblin A<|\"|>,roll:18}",
            "]}"
        );
        let calls = parse_leaked_tool_calls(raw);
        assert_eq!(calls.len(), 1, "expected exactly one tool call");
        let (name, args) = &calls[0];
        assert_eq!(name, "start_combat");
        // args must be valid JSON with the string values cleaned up.
        let v: serde_json::Value = serde_json::from_str(args).expect("args must be valid JSON");
        let entries = v["initiative_entries"].as_array().expect("array");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["id"], "hero");
        assert_eq!(entries[0]["name"], "Hero the Fighter");
        assert_eq!(entries[0]["ac"], 16);
        assert_eq!(entries[1]["id"], "goblin_a");
    }

    #[test]
    fn parse_leaked_tool_calls_ignores_plain_text() {
        assert!(
            parse_leaked_tool_calls("The goblin lunges at you with a rusty dagger.").is_empty()
        );
    }

    #[test]
    fn quote_bare_keys_leaves_quoted_strings_untouched() {
        // A comma inside a string value must not be treated as a key boundary.
        let input = r#"{name:"Smith, the Bold",hp:10}"#;
        let out = quote_bare_keys(input);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(v["name"], "Smith, the Bold");
        assert_eq!(v["hp"], 10);
    }

    #[test]
    fn build_chat_options_some_when_reasoning_set() {
        use crate::provider::ReasoningSpec;
        let opts = build_chat_options(Some(ReasoningSpec::Medium));
        assert!(
            opts.is_some(),
            "expected Some(ChatOptions) for ReasoningSpec::Medium"
        );
    }
}
