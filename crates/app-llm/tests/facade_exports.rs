use app_llm::{
    Capabilities, ChatChunk, ChatMessage, ChatRequest, ChunkStream, FinishReason, LlmError,
    LlmProvider, MessagePart, ReasoningSpec, Tool, ToolCall, ToolResult,
};

#[test]
fn legacy_root_exports_remain_available() {
    fn assert_sized<T: Sized>() {}

    assert_sized::<Capabilities>();
    assert_sized::<ChatChunk>();
    assert_sized::<ChatMessage>();
    assert_sized::<ChatRequest>();
    assert_sized::<FinishReason>();
    assert_sized::<MessagePart>();
    assert_sized::<ReasoningSpec>();
    assert_sized::<Tool>();
    assert_sized::<ToolCall>();
    assert_sized::<ToolResult>();
    assert_sized::<ChunkStream>();
    assert_sized::<LlmError>();
    assert_sized::<Box<dyn LlmProvider>>();
}
