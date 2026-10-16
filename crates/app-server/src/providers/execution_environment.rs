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
        let routing = url
            .query_pairs()
            .filter(|(name, _)| {
                !matches!(
                    name.to_ascii_lowercase().as_str(),
                    "api_key"
                        | "api-key"
                        | "key"
                        | "token"
                        | "access_token"
                        | "authorization"
                        | "auth"
                        | "password"
                )
            })
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        url.set_query(None);
        if !routing.is_empty() {
            url.query_pairs_mut().extend_pairs(routing);
        }
        url.set_fragment(None);
        // A trailing slash and routing parameters can change the executed endpoint.
        Some(url.to_string())
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
        let b = OpenAICompatProvider::new("https://example.test/v1/".into(), "key-b".into());
        let original = environment(&a, "a".into());
        assert_eq!(original, environment(&b, "a".into()));
        assert!(!original.provider.unwrap().contains("secret"));
        assert_ne!(environment(&a, "a".into()), environment(&b, "b".into()));
        assert_eq!(
            environment(&MistralrsLocalProvider::new(1234, "a".into()), "a".into()),
            environment(&MistralrsLocalProvider::new(5678, "a".into()), "a".into())
        );
    }
    #[test]
    fn preserves_endpoint_paths_and_noncredential_routing_parameters() {
        let plain = OpenAICompatProvider::new("https://example.test/v1/".into(), "fixture".into());
        for url in [
            "https://example.test/v1",
            "https://example.test/v1/?tenant=other",
            "https://example.test/v2/",
        ] {
            let other = OpenAICompatProvider::new(url.into(), "fixture".into());
            assert_ne!(
                environment(&plain, "a".into()),
                environment(&other, "a".into())
            );
        }
    }
}
