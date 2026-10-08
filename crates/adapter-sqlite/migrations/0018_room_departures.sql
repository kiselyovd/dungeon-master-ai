CREATE TABLE room_departure_receipts (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    request_id TEXT NOT NULL CHECK(length(request_id) BETWEEN 1 AND 64),
    request_json TEXT NOT NULL,
    receipt_json TEXT NOT NULL,
    PRIMARY KEY(room_id, member_id, request_id)
);

CREATE TABLE room_return_receipts (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    request_id TEXT NOT NULL CHECK(length(request_id) BETWEEN 1 AND 64),
    request_json TEXT NOT NULL,
    PRIMARY KEY(room_id, member_id, request_id)
);
