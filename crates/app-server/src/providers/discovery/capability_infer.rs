use app_llm::Capabilities;

/// Pattern-driven capability inference for models that don't carry explicit
/// caps in their discovery response. Used by HfHubSearch (HF Hub doesn't
/// return caps), OpenAIV1Models (skinny /v1/models response shape), and as
/// the fallback when an explicit value isn't available.
pub fn infer_capabilities(provider_id: &str, model_id: &str, tags: &[String]) -> Capabilities {
    let lc = model_id.to_ascii_lowercase();
    let mut caps = Capabilities {
        vision_input: false,
        reasoning: false,
        tool_calls: true,
        streaming: true,
    };
    match provider_id {
        "anthropic"
            if lc.contains("opus-4") || lc.contains("sonnet-4") || lc.contains("haiku-4") =>
        {
            caps.vision_input = true;
            caps.reasoning = true;
        }
        "anthropic" => {}
        "openai" | "openai-compat" => {
            caps.vision_input =
                lc.contains("gpt-4o") || lc.contains("gpt-5") || lc.starts_with("o4");
            caps.reasoning = lc.starts_with("o1") || lc.starts_with("o3") || lc.starts_with("o4");
        }
        "local-mistralrs" => {
            let tag_vl = tags
                .iter()
                .any(|t| t == "vision-language" || t == "vl" || t.contains("multimodal"));
            caps.vision_input = tag_vl || lc.contains("-vl") || lc.contains("vision");
            caps.reasoning = lc.contains("qwen3") || tags.iter().any(|t| t == "thinking");
            caps.tool_calls = lc.contains("instruct")
                || lc.contains("qwen3")
                || tags.iter().any(|t| t == "conversational");
        }
        _ => {}
    }
    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_opus_4_inferred_all_true() {
        let caps = infer_capabilities("anthropic", "claude-opus-4-7", &[]);
        assert!(caps.vision_input && caps.reasoning && caps.tool_calls && caps.streaming);
    }

    #[test]
    fn openai_compat_gpt_5_inferred_vision_only() {
        let caps = infer_capabilities("openai-compat", "gpt-5", &[]);
        assert!(caps.vision_input);
        assert!(!caps.reasoning);
    }

    #[test]
    fn openai_compat_o3_mini_inferred_reasoning() {
        let caps = infer_capabilities("openai-compat", "o3-mini", &[]);
        assert!(caps.reasoning);
        assert!(!caps.vision_input);
    }

    #[test]
    fn local_mistralrs_qwen3_inferred_reasoning_no_vision_without_tag() {
        let caps = infer_capabilities("local-mistralrs", "qwen3-7b-instruct", &[]);
        assert!(caps.reasoning);
        assert!(!caps.vision_input);
    }

    #[test]
    fn local_mistralrs_qwen3_with_vision_language_tag_gets_vision() {
        let tags = vec!["vision-language".to_string()];
        let caps = infer_capabilities("local-mistralrs", "qwen3-7b-vl", &tags);
        assert!(caps.vision_input);
        assert!(caps.reasoning);
    }

    #[test]
    fn unknown_provider_unknown_model_conservative_defaults() {
        let caps = infer_capabilities("unknown", "unknown", &[]);
        assert!(!caps.vision_input);
        assert!(!caps.reasoning);
        assert!(caps.tool_calls);
        assert!(caps.streaming);
    }
}
