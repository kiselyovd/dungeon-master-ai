CREATE TABLE room_human_command_manifests (
    room_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    manifest_json TEXT NOT NULL CHECK(json_valid(manifest_json)),
    PRIMARY KEY(room_id,member_id,turn_id),
    FOREIGN KEY(room_id,member_id,turn_id) REFERENCES room_dm_turns(room_id,member_id,turn_id) ON DELETE CASCADE
);
CREATE TABLE room_human_command_bindings (
    room_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    command_id INTEGER NOT NULL CHECK(command_id>0),
    step INTEGER NOT NULL CHECK(step>0),
    nonce TEXT NOT NULL,
    binding_json TEXT NOT NULL CHECK(json_valid(binding_json)),
    actor_token TEXT,
    refusal_code TEXT,
    PRIMARY KEY(room_id,member_id,turn_id,tool_id),
    UNIQUE(room_id,member_id,turn_id,command_id,step),
    UNIQUE(room_id,member_id,turn_id,nonce),
    FOREIGN KEY(room_id,member_id,turn_id) REFERENCES room_human_command_manifests(room_id,member_id,turn_id) ON DELETE CASCADE,
    FOREIGN KEY(room_id,member_id,turn_id,tool_id) REFERENCES room_agent_tools(room_id,member_id,turn_id,tool_id) ON DELETE CASCADE
);
