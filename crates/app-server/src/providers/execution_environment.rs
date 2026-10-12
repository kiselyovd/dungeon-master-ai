use app_application::{agent::operations::ExecutionEnvironment, ports::llm::LlmProvider};
use sha2::{Digest, Sha256};

pub(crate) fn environment(provider: &dyn LlmProvider, model: String) -> ExecutionEnvironment {
    let origin = provider.execution_origin().and_then(|origin| {
        if provider.name() != "openai-compat" {
            return Some(origin);
        }
        let mut url = reqwest::Url::parse(&origin).ok()?;
        // Credential rotation is repair, not a different execution environment.
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_query(None);
        url.set_fragment(None);
        Some(url.to_string().trim_end_matches('/').to_owned())
    });
    ExecutionEnvironment {
        provider: origin.map(|origin| {
            Sha256::digest(format!("{}:{origin}", provider.name()).as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        }),
        model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_llm::{MistralrsLocalProvider, OpenAICompatProvider};
    #[test]
    fn credentials_and_native_ports_are_not_execution_identity() {
        let a = OpenAICompatProvider::new(
            "https://user:secret@example.test/v1/?token=secret".into(),
            "key-a".into(),
        );
        let b = OpenAICompatProvider::new("https://example.test/v1".into(), "key-b".into());
        let original = environment(&a, "a".into());
        assert_eq!(original, environment(&b, "a".into()));
        assert!(!original.provider.unwrap().contains("secret"));
        assert_ne!(environment(&a, "a".into()), environment(&b, "b".into()));
        assert_eq!(
            environment(&MistralrsLocalProvider::new(1234, "a".into()), "a".into()),
            environment(&MistralrsLocalProvider::new(5678, "a".into()), "a".into())
        );
    }
}
