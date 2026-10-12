-- Selected appearance is separate from immutable bytes and from authoritative geometry.
CREATE TABLE current_map_media (
    campaign_id TEXT PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    reference_json TEXT NOT NULL CHECK (json_valid(reference_json)),
    request_id TEXT NOT NULL
);
