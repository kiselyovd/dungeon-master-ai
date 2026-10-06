CREATE TABLE campaign_adventures (
    campaign_id TEXT PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
    state_json TEXT NOT NULL CHECK(json_valid(state_json))
);
CREATE TABLE adventure_operations (
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    causation_id TEXT NOT NULL,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    state_json TEXT NOT NULL CHECK(json_valid(state_json)),
    PRIMARY KEY(campaign_id, causation_id)
);
