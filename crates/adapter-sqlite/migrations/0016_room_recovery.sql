CREATE TABLE room_recovery_receipts (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    request_id TEXT NOT NULL,
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json)),
    reset_cursor INTEGER NOT NULL CHECK(reset_cursor>=0),
    PRIMARY KEY(room_id,member_id,request_id)
);
CREATE TRIGGER room_recovery_completed AFTER INSERT ON room_recovery_receipts BEGIN
    INSERT INTO room_changes(room_id,kind,payload_json)
    SELECT room_id,'state',json_object('operation','recovery','member_id',NEW.member_id,'receipt',json(NEW.receipt_json))
    FROM coop_rooms WHERE active=1 AND room_id=NEW.room_id;
END;
