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
async fn quantified_source_expands_only_with_complete_group_and_semantic_count_approval() {
    let quote = "Жду минуту дважды";
    let commands = (1..=2)
        .map(|index| {
            let mut value = command(quote, 0);
            value["repetition"] = json!({"index":index,"total":2});
            value
        })
        .collect::<Vec<_>>();
    let proposal = json!({"commands":commands,"constraints":[]});
    let p = provider(vec![
        (proposal.clone(), FinishReason::Stop),
        (json!({"valid":true}), FinishReason::Stop),
    ]);
    let manifest = LlmHumanCommandPlanner::new(p.clone(), "script".into())
        .plan(input(quote))
        .await
        .unwrap();
    assert_eq!(manifest.commands.len(), 2);
    assert_eq!(manifest.commands[0].source, manifest.commands[1].source);
    assert_eq!(manifest.commands[1].repetition.as_ref().unwrap().index, 2);
    assert_eq!(
        p.requests.lock().unwrap().len(),
        2,
        "independent count approval is mandatory"
    );
    let p = provider(vec![
        (proposal.clone(), FinishReason::Stop),
        (json!({"valid":false}), FinishReason::Stop),
    ]);
    assert!(LlmHumanCommandPlanner::new(p, "script".into())
        .plan(input(quote))
        .await
        .is_err());
    let mut incomplete = proposal;
    incomplete["commands"].as_array_mut().unwrap().pop();
    let p = provider(vec![
        (incomplete.clone(), FinishReason::Stop),
        (incomplete, FinishReason::Stop),
    ]);
    assert!(LlmHumanCommandPlanner::new(p.clone(), "script".into())
        .plan(input(quote))
        .await
        .is_err());
    assert_eq!(p.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_prohibition_cannot_also_be_an_unresolved_command_and_duration_is_not_repetition() {
    let text = "Жду минуту дважды. Ход не завершаю.";
    let mut commands = (1..=2)
        .map(|index| {
            let mut value = command("Жду минуту дважды", 0);
            value["repetition"] = json!({"index":index,"total":2});
            value
        })
        .collect::<Vec<_>>();
    commands.push(json!({"source":{"quote":"Ход не завершаю","occurrence":0},"tool_name":null,"args_json":null,"unresolved":"Prohibit combat_action with kind: end_turn","condition":null,"repetition":null}));
    let invalid =
        json!({"commands":commands,"constraints":[{"quote":"Ход не завершаю","occurrence":0}]});
    let p = provider(vec![
        (invalid.clone(), FinishReason::Stop),
        (invalid, FinishReason::Stop),
    ]);
    assert!(LlmHumanCommandPlanner::new(p.clone(), "script".into())
        .plan(input(text))
        .await
        .is_err());
    assert_eq!(
        p.requests.lock().unwrap().len(),
        2,
        "a still-invalid structural repair never reaches semantic approval"
    );
    let mut wait = command("Жду две минуты", 0);
    wait["args_json"] = json!(json!({"kind":"wait","minutes":2}).to_string());
    let p = provider(vec![
        (
            json!({"commands":[wait],"constraints":[{"quote":"Ход не завершаю","occurrence":0}]}),
            FinishReason::Stop,
        ),
        (json!({"valid":true}), FinishReason::Stop),
    ]);
    let result = LlmHumanCommandPlanner::new(p, "script".into())
        .plan(input("Жду две минуты. Ход не завершаю."))
        .await
        .unwrap();
    assert_eq!(result.commands.len(), 1);
    assert!(result.commands[0].repetition.is_none());
    assert_eq!(
        result.commands[0].goal,
        HumanCommandGoal::Exact {
            action: HumanAction::Rest(app_application::models::rest::RestIntent::Wait {
                minutes: 2
            })
        }
    );
    assert_eq!(result.constraints.len(), 1);
}

#[tokio::test]
async fn one_structural_repair_preserves_original_scope_and_still_needs_semantic_approval() {
    let text = "Жду минуту. Жду минуту. Ход не завершаю.";
    let original = input(text);
    let malformed = json!({"commands":[command("Жду минуту",0),command("Жду минуту",2)],"constraints":[{"quote":"Ход не завершаю","occurrence":2}]});
    let corrected = json!({"commands":[command("Жду минуту",0),command("Жду минуту",1)],"constraints":[{"quote":"Ход не завершаю","occurrence":0}]});
    for valid in [false, true] {
        let p = provider(vec![
            (malformed.clone(), FinishReason::Stop),
            (corrected.clone(), FinishReason::Stop),
            (json!({"valid":valid}), FinishReason::Stop),
        ]);
        let result = LlmHumanCommandPlanner::new(p.clone(), "saved-model".into())
            .plan(original.clone())
            .await;
        assert_eq!(result.is_ok(), valid);
        if let Ok(manifest) = result {
            assert_eq!(manifest.scope, original.scope);
            assert_eq!(manifest.commands.len(), 2);
            assert!(manifest.commands.iter().all(|c| c.repetition.is_none()));
            assert_ne!(
                manifest.commands[0].source.start,
                manifest.commands[1].source.start
            );
            assert_eq!(manifest.constraints[0].quote, "Ход не завершаю");
        }
        let requests = p.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|(r, _)| r.tools.is_empty() && r.model == "saved-model"));
        let ChatMessage::User { parts } = &requests[1].0.messages[0] else {
            panic!("quoted repair input")
        };
        let MessagePart::Text { text: payload } = &parts[0] else {
            panic!("text input")
        };
        let payload: Value = serde_json::from_str(payload).unwrap();
        assert_eq!(payload["original"]["player_message"], text);
        assert_eq!(
            payload["rejected_proposal"]["commands"][1]["source"]["occurrence"],
            2
        );
        assert!(requests[2]
            .0
            .system_prompt
            .as_ref()
            .unwrap()
            .contains("Independently"));
    }
    for reason in [FinishReason::Length, FinishReason::Stop] {
        let p = provider(vec![
            (malformed.clone(), FinishReason::Stop),
            (malformed.clone(), reason),
        ]);
        assert!(LlmHumanCommandPlanner::new(p.clone(), "saved-model".into())
            .plan(original.clone())
            .await
            .is_err());
        assert_eq!(
            p.requests.lock().unwrap().len(),
            2,
            "no unbounded repair loop or unchecked manifest"
        );
    }
}

#[tokio::test]
async fn source_errors_never_mask_invalid_arguments_in_any_command() {
    for bad_position in [0, 1] {
        let mut commands = vec![command("missing quote", 0), command("Жду минуту", 0)];
        commands[bad_position]["tool_name"] = json!("apply_damage");
        commands[bad_position]["args_json"] = json!(json!({"amount":999}).to_string());
        let p = provider(vec![(
            json!({"commands":commands,"constraints":[]}),
            FinishReason::Stop,
        )]);
        assert!(LlmHumanCommandPlanner::new(p.clone(), "saved-model".into())
            .plan(input("Жду минуту"))
            .await
            .is_err());
        assert_eq!(
            p.requests.lock().unwrap().len(),
            1,
            "no structural repair or semantic approval when ANY typed argument is invalid"
        );
    }
}
#[tokio::test]
async fn compound_adventure_requires_full_wrapped_arguments_and_preserves_negative_constraint() {
    let text = "Иду на старый причал и осматриваю его. Дальше никуда не иду.";
    let proposal = json!({"commands":[{"source":{"quote":"Иду на старый причал","occurrence":0},"tool_name":"adventure_action","args_json":json!({"action":{"kind":"move","location":"quay"}}).to_string(),"unresolved":null,"condition":null},{"source":{"quote":"осматриваю его","occurrence":0},"tool_name":"adventure_action","args_json":json!({"action":{"kind":"inspect_quay"}}).to_string(),"unresolved":null,"condition":null}],"constraints":[{"quote":"Дальше никуда не иду","occurrence":0}]});
    let p = provider(vec![
        (proposal.clone(), FinishReason::Stop),
        (json!({"valid":true}), FinishReason::Stop),
    ]);
    let manifest = LlmHumanCommandPlanner::new(p, "script".into())
        .plan(input(text))
        .await
        .unwrap();
    assert_eq!(manifest.commands.len(), 2);
    assert_eq!(manifest.constraints.len(), 1);
    let mut bad = proposal;
    bad["commands"][0]["args_json"] = json!(json!({"kind":"move","location":"quay"}).to_string());
    let p = provider(vec![(bad, FinishReason::Stop)]);
    assert!(LlmHumanCommandPlanner::new(p.clone(), "script".into())
        .plan(input(text))
        .await
        .is_err());
    assert_eq!(
        p.requests.lock().unwrap().len(),
        1,
        "bad arguments never reach semantic approval or execution"
    );
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
            2,
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
