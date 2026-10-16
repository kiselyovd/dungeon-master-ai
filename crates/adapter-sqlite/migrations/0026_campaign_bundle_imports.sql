-- A lost native response may retry the same import, never create a second copy.
CREATE TABLE campaign_bundle_imports (
    operation_id TEXT PRIMARY KEY NOT NULL,
    integrity_sha256 TEXT NOT NULL CHECK(length(integrity_sha256)=64),
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    result_json TEXT NOT NULL CHECK(json_valid(result_json))
);
