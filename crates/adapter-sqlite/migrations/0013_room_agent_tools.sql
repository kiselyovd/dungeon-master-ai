CREATE TABLE room_agent_tools (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    command_json TEXT NOT NULL CHECK(json_valid(command_json)),
    reset_cursor INTEGER NOT NULL,
    prepared_json TEXT NOT NULL CHECK(json_valid(prepared_json)),
    PRIMARY KEY(room_id,member_id,turn_id,tool_id)
);
CREATE TABLE room_agent_messages (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    message_id TEXT NOT NULL,
    message_json TEXT NOT NULL CHECK(json_valid(message_json)),
    reset_cursor INTEGER NOT NULL,
    PRIMARY KEY(room_id,member_id,turn_id,sequence)
);
