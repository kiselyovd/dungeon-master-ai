CREATE TABLE room_dm_turns (
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    member_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    room_epoch INTEGER NOT NULL,
    reset_cursor INTEGER NOT NULL,
    process_owner TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN('running','interrupted','complete','superseded')),
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    PRIMARY KEY(room_id,member_id,turn_id)
);
CREATE UNIQUE INDEX room_dm_one_pending_turn ON room_dm_turns(campaign_id) WHERE status IN('running','interrupted');
CREATE TABLE room_dm_events (
    room_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    event_json TEXT NOT NULL CHECK(json_valid(event_json)),
    PRIMARY KEY(room_id,member_id,turn_id,sequence),
    FOREIGN KEY(room_id,member_id,turn_id) REFERENCES room_dm_turns(room_id,member_id,turn_id) ON DELETE CASCADE
);
CREATE TRIGGER room_dm_turn_created AFTER INSERT ON room_dm_turns BEGIN
    INSERT INTO room_changes(room_id,kind) VALUES(NEW.room_id,'state');
END;
CREATE TRIGGER room_dm_turn_updated AFTER UPDATE ON room_dm_turns BEGIN
    INSERT INTO room_changes(room_id,kind) VALUES(NEW.room_id,'state');
END;
CREATE TRIGGER room_dm_epoch_invalidated AFTER UPDATE ON coop_rooms BEGIN
    UPDATE room_dm_turns SET status='superseded' WHERE room_id=NEW.room_id AND status IN('running','interrupted') AND (NEW.active=0 OR room_epoch!=json_extract(NEW.record_json,'$.epoch'));
END;
CREATE TRIGGER room_dm_timeline_invalidated AFTER INSERT ON snapshot_restores BEGIN
    UPDATE room_dm_turns SET status='superseded' WHERE status IN('running','interrupted') AND room_id IN(SELECT room_id FROM coop_rooms WHERE session_id=NEW.session_id);
END;
