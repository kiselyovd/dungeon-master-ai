CREATE TABLE coop_rooms (
    room_id TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL REFERENCES campaign_parties(campaign_id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    active INTEGER NOT NULL CHECK(active IN(0,1)),
    record_json TEXT NOT NULL CHECK(json_valid(record_json))
);
CREATE UNIQUE INDEX coop_one_active_room ON coop_rooms(campaign_id,session_id) WHERE active=1;
CREATE TABLE room_creation_receipts (
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    causation_id TEXT NOT NULL,
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    signature TEXT NOT NULL,
    PRIMARY KEY(campaign_id,causation_id)
);
CREATE TABLE room_join_receipts (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    join_id TEXT NOT NULL,
    record_json TEXT NOT NULL CHECK(json_valid(record_json)),
    PRIMARY KEY(room_id,join_id)
);
