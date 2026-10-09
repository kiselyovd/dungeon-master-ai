use app_application::{
    agent::{command_planning::*, human_commands::*},
    models::chat::*,
    ports::llm::*,
};
use futures::StreamExt;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
struct Provider {
    replies: Mutex<VecDeque<(Value, FinishReason)>>,
    requests: Mutex<Vec<(ChatRequest, Value)>>,
}
#[async_trait::async_trait]
impl LlmProvider for Provider {
    async fn stream_chat(&self, _: ChatRequest) -> Result<ChunkStream, LlmError> {
        panic!("structured output required")
    }
    async fn stream_chat_json(
        &self,
        request: ChatRequest,
        schema: Value,
    ) -> Result<ChunkStream, LlmError> {
        assert_strict_schema(&schema);
        self.requests.lock().unwrap().push((request, schema));
        let (text, reason) = self.replies.lock().unwrap().pop_front().unwrap();
        Ok(futures::stream::iter(vec![
            Ok(ChatChunk::TextDelta {
                text: text.to_string(),
            }),
            Ok(ChatChunk::Done { reason }),
        ])
        .boxed())
    }
    fn name(&self) -> &'static str {
        "script"
    }
    fn active_model(&self) -> &str {
        "script"
    }
    fn capabilities_for_model(&self, _: &str) -> Capabilities {
        Default::default()
    }
}
fn assert_strict_schema(schema: &Value) {
    assert!(
        schema.get("oneOf").is_none(),
        "strict Structured Outputs supports anyOf, not oneOf"
    );
    if schema["type"] == "object" {
        assert_eq!(schema["additionalProperties"], false);
        let properties = schema["properties"]
            .as_object()
            .expect("closed typed object");
        let required = schema["required"]
            .as_array()
            .expect("all properties required");
        assert_eq!(required.len(), properties.len());
        for (name, child) in properties {
            assert!(required.contains(&json!(name)));
            assert_strict_schema(child);
        }
    }
    if let Some(types) = schema["type"].as_array() {
        assert!(
            !types.contains(&json!("object")),
            "use typed anyOf for nullable objects"
        );
    }
    if let Some(items) = schema.get("items") {
        assert_strict_schema(items);
    }
    if let Some(variants) = schema["anyOf"].as_array() {
        for variant in variants {
            assert_strict_schema(variant);
        }
    }
}
fn input(text: &str) -> CommandPlanningInput {
    CommandPlanningInput {
        scope: CommandScope {
            request_id: Uuid::new_v4(),
            campaign_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            authority: CommandAuthority::Solo {
                restore_generation: 0,
            },
        },
        player_message: text.into(),
        public_context: json!({"hero_name":"Mira"}),
        tools: vec![],
    }
}
fn provider(replies: Vec<(Value, FinishReason)>) -> Arc<Provider> {
    Arc::new(Provider {
        replies: Mutex::new(replies.into()),
        requests: Mutex::new(vec![]),
    })
}
fn command(quote: &str, occurrence: usize) -> Value {
    json!({"source":{"quote":quote,"occurrence":occurrence},"tool_name":"rest_action","args_json":json!({"kind":"wait","minutes":1}).to_string(),"unresolved":null,"condition":null})
}
#[tokio::test]
async fn compound_plan_preserves_repeated_commands_and_prohibition_and_is_independently_checked() {
    let text = "Жду минуту, жду минуту. Ход не заканчиваю";
    let p = provider(vec![
        (
            json!({"commands":[command("Жду минуту",0),command("жду минуту",0)],"constraints":[{"quote":"Ход не заканчиваю","occurrence":0}]}),
            FinishReason::Stop,
        ),
        (json!({"valid":true}), FinishReason::Stop),
    ]);
    let saved = input(text);
    let result = LlmHumanCommandPlanner::new(p.clone(), "chosen-model".into())
        .plan(saved.clone())
        .await
        .unwrap();
    assert_eq!(result.scope, saved.scope);
    assert_eq!(result.commands.len(), 2);
    assert_eq!(result.constraints.len(), 1);
    let requests = p.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|(request, _)| request.model == "chosen-model"
            && request.tools.is_empty()
            && request.reasoning.is_none()));
    assert!(requests[1]
        .0
        .system_prompt
        .as_ref()
        .unwrap()
        .contains("Independently"));
}
#[tokio::test]
async fn invalid_quote_raw_dm_effect_or_false_verdict_never_produce_a_manifest() {
    for (proposal, decision, count) in [
        (
            json!({"commands":[command("Не было этих слов",0)],"constraints":[]}),
            json!({"valid":true}),
            1,
        ),
        (
            json!({"commands":[{"source":{"quote":"Жду минуту","occurrence":0},"tool_name":"apply_damage","args_json":json!({"target":"x","amount":99}).to_string(),"unresolved":null,"condition":null}],"constraints":[]}),
            json!({"valid":true}),
            1,
        ),
        (
            json!({"commands":[command("Жду минуту",0)],"constraints":[]}),
            json!({"valid":false}),
            2,
        ),
    ] {
        let p = provider(vec![
            (proposal, FinishReason::Stop),
            (decision, FinishReason::Stop),
        ]);
        assert!(LlmHumanCommandPlanner::new(p.clone(), "script".into())
            .plan(input("Жду минуту"))
            .await
            .is_err());
        assert_eq!(p.requests.lock().unwrap().len(), count);
    }
}
#[tokio::test]
async fn future_target_stays_unresolved_and_specific_refusal_condition_is_retained() {
    let p = provider(vec![
        (
            json!({"commands":[command("Жду минуту",0),{"source":{"quote":"атакую появившегося врага","occurrence":0},"tool_name":null,"args_json":null,"unresolved":"Attack the enemy who appears after waiting with my equipped sword","condition":{"command_id":1,"predicate":{"attempt_refused_because":{"code":"rest_unsafe"}}}}],"constraints":[]}),
            FinishReason::Stop,
        ),
        (json!({"valid":true}), FinishReason::Stop),
    ]);
    let result = LlmHumanCommandPlanner::new(p, "script".into())
        .plan(input(
            "Жду минуту; если отдых небезопасен, атакую появившегося врага",
        ))
        .await
        .unwrap();
    assert!(matches!(
        result.commands[1].goal,
        HumanCommandGoal::NeedsResolution { .. }
    ));
    assert_eq!(
        result.commands[1].condition.as_ref().unwrap().predicate,
        CommandPredicate::AttemptRefusedBecause {
            code: "rest_unsafe".into()
        }
    );
}
#[tokio::test]
async fn truncated_generation_or_verification_is_not_accepted() {
    for stage in 0..2 {
        let p = provider(vec![
            (
                json!({"commands":[command("Жду минуту",0)],"constraints":[]}),
                if stage == 0 {
                    FinishReason::Length
                } else {
                    FinishReason::Stop
                },
            ),
            (json!({"valid":true}), FinishReason::Length),
        ]);
        assert!(LlmHumanCommandPlanner::new(p, "script".into())
            .plan(input("Жду минуту"))
            .await
            .is_err());
    }
}
