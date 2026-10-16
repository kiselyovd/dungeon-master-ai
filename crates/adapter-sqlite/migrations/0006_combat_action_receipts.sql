CREATE TABLE combat_action_receipts (
    encounter_id TEXT NOT NULL REFERENCES combat_encounters(id) ON DELETE CASCADE,
    request_id TEXT NOT NULL,
    signature TEXT NOT NULL CHECK(json_valid(signature)),
    projection_json TEXT NOT NULL CHECK(json_valid(projection_json)),
    created_at TEXT NOT NULL,
    PRIMARY KEY(encounter_id, request_id)
);
