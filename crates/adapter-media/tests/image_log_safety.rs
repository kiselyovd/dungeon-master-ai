#[test]
fn replicate_logs_never_contain_prompts_or_generated_urls() {
    let source = include_str!("../src/image/replicate.rs");

    assert!(!source.contains("prompt = %full_prompt"));
    assert!(!source.contains("url = %url"));
}
