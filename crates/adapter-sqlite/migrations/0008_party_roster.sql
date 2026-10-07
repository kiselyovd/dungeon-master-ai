CREATE TABLE campaign_parties (
    campaign_id TEXT PRIMARY KEY NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    host_member_id TEXT NOT NULL UNIQUE
);
CREATE TABLE party_characters (
    campaign_id TEXT NOT NULL REFERENCES campaign_parties(campaign_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    character_id TEXT NOT NULL,
    character_json TEXT NOT NULL CHECK(json_valid(character_json)),
    PRIMARY KEY(campaign_id,member_id),
    UNIQUE(campaign_id,character_id)
);
CREATE TABLE party_operation_receipts (
    campaign_id TEXT NOT NULL REFERENCES campaign_parties(campaign_id) ON DELETE CASCADE,
    causation_id TEXT NOT NULL,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    PRIMARY KEY(campaign_id,causation_id)
);
