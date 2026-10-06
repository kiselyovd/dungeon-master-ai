CREATE TABLE agent_turn_operations (
    request_id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    owner_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','complete','interrupted')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE agent_turn_events (
    request_id TEXT NOT NULL REFERENCES agent_turn_operations(request_id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK(sequence >= 0),
    event_json TEXT NOT NULL CHECK(json_valid(event_json)),
    PRIMARY KEY(request_id,sequence)
);
CREATE INDEX agent_turn_campaign_status ON agent_turn_operations(campaign_id,status);
