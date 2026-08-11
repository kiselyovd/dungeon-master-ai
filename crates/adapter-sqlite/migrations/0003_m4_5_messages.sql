-- M4.5 messages table: persists chat history per session for resumable campaigns.
-- `parts` is a JSON array of MessagePart objects (text or image with base64 inline).
-- `tool_calls` and `tool_call_id` are populated only for the relevant roles.

CREATE TABLE IF NOT EXISTS messages (
    id           TEXT PRIMARY KEY NOT NULL, -- UUID v4
    session_id   TEXT NOT NULL,
    role         TEXT NOT NULL,             -- system|user|assistant|assistant_with_tool_calls|tool_result
    parts        TEXT NOT NULL,             -- JSON: Vec<MessagePart>
    tool_calls   TEXT,                      -- JSON: Vec<ToolCall>; NULL unless role=assistant_with_tool_calls
    tool_call_id TEXT,                      -- non-NULL when role=tool_result
    is_error     INTEGER NOT NULL DEFAULT 0,-- 0 or 1; meaningful only when role=tool_result
    created_at   TEXT NOT NULL              -- RFC3339 UTC
);

CREATE INDEX IF NOT EXISTS messages_session_created
    ON messages(session_id, created_at);
