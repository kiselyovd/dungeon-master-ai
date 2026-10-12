-- Immutable content is deduplicated. References retain distinct ownership/origin.
CREATE TABLE media_content (
    sha256 TEXT PRIMARY KEY CHECK (length(sha256) = 64),
    descriptor_json TEXT NOT NULL,
    payload BLOB NOT NULL CHECK (length(payload) > 0 AND length(payload) <= 8388608)
);
CREATE TABLE media_assets (
    asset_id TEXT PRIMARY KEY,
    sha256 TEXT NOT NULL REFERENCES media_content(sha256),
    reference_json TEXT NOT NULL,
    owner_kind TEXT NOT NULL CHECK (owner_kind IN ('draft', 'campaign', 'room_member')),
    owner_id TEXT NOT NULL,
    actor_id TEXT
);
CREATE INDEX media_assets_owner ON media_assets(owner_kind, owner_id);
CREATE TABLE media_bindings (
    asset_id TEXT NOT NULL REFERENCES media_assets(asset_id),
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    actor_id TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (asset_id, campaign_id, actor_id)
);
