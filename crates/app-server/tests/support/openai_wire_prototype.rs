//! Native OpenAI SSE prototype; no production HTTP integration yet.
use app_application::models::chat::{
    ChatChunk, ChatMessage, ChatRequest, FinishReason, MessagePart,
};
use app_application::ports::llm::LlmError;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn request_body(request: ChatRequest) -> Value {
    let mut messages = Vec::new();
    if let Some(content) = request.system_prompt {
        messages.push(json!({"role":"system","content":content}));
    }
    messages.extend(request.messages.into_iter().map(|message|match message{
        ChatMessage::System{content}=>json!({"role":"system","content":content}),
        ChatMessage::User{parts}=>json!({"role":"user","content":parts.into_iter().map(|part|match part{MessagePart::Text{text}=>json!({"type":"text","text":text}),MessagePart::Image{mime,data_b64,..}=>json!({"type":"image_url","image_url":{"url":format!("data:{mime};base64,{data_b64}")}})}).collect::<Vec<_>>()}),
        ChatMessage::Assistant{content}=>json!({"role":"assistant","content":content}),
        ChatMessage::AssistantWithToolCalls{content,tool_calls}=>json!({"role":"assistant","content":content,"tool_calls":tool_calls.into_iter().map(|call|json!({"id":call.id,"type":"function","function":{"name":call.name,"arguments":call.args.to_string()}})).collect::<Vec<_>>()}),
        ChatMessage::ToolResult(result)=>json!({"role":"tool","tool_call_id":result.tool_call_id,"content":result.content}),
    }));
    let mut body = json!({"model":request.model,"messages":messages,"stream":true});
    if let Some(max) = request.max_tokens {
        body["max_tokens"] = max.into();
    }
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(reasoning) = request.reasoning {
        body["reasoning_effort"] = json!(reasoning);
    }
    if !request.tools.is_empty() {
        body["tools"]=json!(request.tools.into_iter().map(|tool|json!({"type":"function","function":{"name":tool.name,"description":tool.description,"parameters":tool.parameters}})).collect::<Vec<_>>());
    }
    body
}

#[derive(Default)]
struct ToolDelta {
    id: Option<String>,
    name: String,
    arguments: String,
}
#[derive(Default)]
pub(crate) struct OpenAiDecoder {
    buffer: Vec<u8>,
    calls: BTreeMap<u64, ToolDelta>,
    done: bool,
}
fn invalid() -> LlmError {
    LlmError::InvalidResponse("invalid_openai_stream".into())
}
impl OpenAiDecoder {
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Vec<ChatChunk>, LlmError> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > 2 * 1024 * 1024 {
            return Err(invalid());
        }
        let mut output = Vec::new();
        loop {
            let boundary = self
                .buffer
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|at| (at, 2));
            let crlf = self
                .buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|at| (at, 4));
            let Some((at, len)) = boundary.into_iter().chain(crlf).min_by_key(|(at, _)| *at) else {
                break;
            };
            let raw = self.buffer.drain(..at + len).collect::<Vec<_>>();
            let frame = std::str::from_utf8(&raw[..at]).map_err(|_| invalid())?;
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
                .collect::<Vec<_>>()
                .join("\n");
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                if !self.done {
                    return Err(invalid());
                }
                continue;
            }
            if self.done {
                continue;
            }
            let event: Value = serde_json::from_str(&data).map_err(|_| invalid())?;
            if event.get("error").is_some() {
                return Err(invalid());
            }
            let Some(choices) = event["choices"].as_array() else {
                return Err(invalid());
            };
            for choice in choices
                .iter()
                .filter(|choice| choice["index"].as_u64().unwrap_or(0) == 0)
            {
                if let Some(text) = choice["delta"]["reasoning_content"].as_str() {
                    output.push(ChatChunk::ThinkingDelta { text: text.into() });
                }
                if let Some(text) = choice["delta"]["content"].as_str() {
                    output.push(ChatChunk::TextDelta { text: text.into() });
                }
                if let Some(calls) = choice["delta"]["tool_calls"].as_array() {
                    for call in calls {
                        let index = call["index"]
                            .as_u64()
                            .filter(|index| *index < 64)
                            .ok_or_else(invalid)?;
                        let entry = self.calls.entry(index).or_default();
                        if let Some(id) = call["id"].as_str() {
                            if entry.id.as_ref().is_some_and(|old| old != id) {
                                return Err(invalid());
                            }
                            entry.id = Some(id.into());
                        }
                        if let Some(name) = call["function"]["name"].as_str() {
                            entry.name.push_str(name);
                        }
                        if let Some(arguments) = call["function"]["arguments"].as_str() {
                            entry.arguments.push_str(arguments);
                        }
                        if entry.arguments.len() > 2 * 1024 * 1024 || entry.name.len() > 256 {
                            return Err(invalid());
                        }
                    }
                }
                if let Some(reason) = choice["finish_reason"].as_str() {
                    if !matches!(reason, "stop" | "tool_calls" | "function_call") {
                        return Err(invalid());
                    }
                    let had_calls = !self.calls.is_empty();
                    if self
                        .calls
                        .values()
                        .filter_map(|call| call.id.as_deref())
                        .collect::<BTreeSet<_>>()
                        .len()
                        != self.calls.len()
                    {
                        return Err(invalid());
                    }
                    // Validate every tool before delivering any completed call.
                    // A malformed parallel peer must not permit partial effects.
                    for call in self.calls.values() {
                        if call
                            .id
                            .as_ref()
                            .is_none_or(|id| id.is_empty() || id.len() > 256)
                            || call.name.is_empty()
                            || serde_json::from_str::<Value>(&call.arguments).is_err()
                        {
                            return Err(invalid());
                        }
                    }
                    for (_, call) in std::mem::take(&mut self.calls) {
                        let id = call.id.ok_or_else(invalid)?;
                        output.push(ChatChunk::ToolCallStart {
                            id: id.clone(),
                            name: call.name,
                        });
                        output.push(ChatChunk::ToolCallArgsDelta {
                            id: id.clone(),
                            args_fragment: call.arguments,
                        });
                        output.push(ChatChunk::ToolCallDone { id });
                    }
                    output.push(ChatChunk::Done {
                        reason: match reason {
                            "tool_calls" | "function_call" => FinishReason::ToolUse,
                            "length" => FinishReason::Length,
                            "stop" if had_calls => FinishReason::ToolUse,
                            "stop" => FinishReason::Stop,
                            _ => FinishReason::Error,
                        },
                    });
                    self.done = true;
                }
            }
        }
        Ok(output)
    }
    pub(crate) fn finish(&self) -> Result<(), LlmError> {
        if self.done {
            Ok(())
        } else {
            Err(invalid())
        }
    }
}
