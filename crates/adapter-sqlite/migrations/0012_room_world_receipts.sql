CREATE TABLE room_world_receipts (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    request_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN('adventure','rest')),
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json)),
    PRIMARY KEY(room_id,member_id,request_id)
);
CREATE TRIGGER room_world_completed AFTER INSERT ON room_world_receipts BEGIN
    INSERT INTO room_changes(room_id,kind,payload_json)
    SELECT room_id,'state',json_object('operation',NEW.kind,'member_id',NEW.member_id,'receipt_revision',json_extract(NEW.receipt_json,'$.receipt_revision'),'rolls',json_extract(NEW.receipt_json,'$.rest_outcome.rolls'))
    FROM coop_rooms WHERE active=1 AND room_id=NEW.room_id;
END;
