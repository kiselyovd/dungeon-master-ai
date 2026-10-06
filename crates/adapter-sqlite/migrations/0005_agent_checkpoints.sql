ALTER TABLE sessions ADD COLUMN restore_generation INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agent_turn_operations ADD COLUMN restore_generation INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agent_turn_operations ADD COLUMN checkpoint_version INTEGER NOT NULL DEFAULT 0;
CREATE TABLE agent_turn_messages (
    request_id TEXT NOT NULL REFERENCES agent_turn_operations(request_id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    message_json TEXT NOT NULL CHECK(json_valid(message_json)),
    PRIMARY KEY(request_id,sequence)
);
CREATE TABLE agent_tool_steps (
    request_id TEXT NOT NULL REFERENCES agent_turn_operations(request_id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL,
    command_json TEXT NOT NULL CHECK(json_valid(command_json)),
    result_json TEXT CHECK(result_json IS NULL OR json_valid(result_json)),
    PRIMARY KEY(request_id,tool_call_id)
);
