CREATE TABLE solo_human_command_manifests (
    request_id TEXT PRIMARY KEY REFERENCES agent_turn_operations(request_id) ON DELETE CASCADE,
    manifest_json TEXT NOT NULL CHECK(json_valid(manifest_json))
);
CREATE TABLE solo_human_command_bindings (
    request_id TEXT NOT NULL,
    tool_call_id TEXT NOT NULL,
    command_id INTEGER NOT NULL CHECK(command_id > 0),
    step INTEGER NOT NULL CHECK(step > 0),
    binding_json TEXT NOT NULL CHECK(json_valid(binding_json)),
    preparation_json TEXT NOT NULL CHECK(json_valid(preparation_json)),
    PRIMARY KEY(request_id,tool_call_id),
    UNIQUE(request_id,command_id,step),
    FOREIGN KEY(request_id) REFERENCES solo_human_command_manifests(request_id) ON DELETE CASCADE,
    FOREIGN KEY(request_id,tool_call_id) REFERENCES agent_tool_steps(request_id,tool_call_id) ON DELETE CASCADE
);
