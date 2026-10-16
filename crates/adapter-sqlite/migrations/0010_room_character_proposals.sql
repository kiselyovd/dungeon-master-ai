CREATE TABLE room_character_proposals (
    proposal_id TEXT PRIMARY KEY NOT NULL,
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    character_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN('pending','accepted','superseded')),
    character_json TEXT NOT NULL CHECK(json_valid(character_json))
);
CREATE UNIQUE INDEX room_one_pending_character ON room_character_proposals(room_id,member_id) WHERE status='pending';
CREATE TABLE room_character_approvals (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    causation_id TEXT NOT NULL,
    proposal_id TEXT NOT NULL REFERENCES room_character_proposals(proposal_id) ON DELETE CASCADE,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    revision INTEGER NOT NULL CHECK(revision >= 0),
    PRIMARY KEY(room_id,causation_id)
);
