CREATE TABLE room_changes (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id TEXT NOT NULL REFERENCES coop_rooms(room_id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN('state','combat','message','timeline_reset')),
    payload_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(payload_json))
);
CREATE INDEX room_changes_cursor ON room_changes(room_id,sequence);
CREATE TABLE shared_messages (
    message_id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE
);
CREATE TABLE room_restore_guard (
    session_id TEXT PRIMARY KEY NOT NULL REFERENCES sessions(id) ON DELETE CASCADE
);
CREATE TRIGGER room_created_update AFTER INSERT ON coop_rooms WHEN NEW.active=1 BEGIN
    INSERT INTO room_changes(room_id,kind) VALUES(NEW.room_id,'state');
END;
CREATE TRIGGER room_metadata_update AFTER UPDATE ON coop_rooms WHEN NEW.active=1 BEGIN
    INSERT INTO room_changes(room_id,kind) VALUES(NEW.room_id,'state');
END;
CREATE TRIGGER room_campaign_update AFTER INSERT ON campaign_events BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_proposal_created AFTER INSERT ON room_character_proposals BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND room_id=NEW.room_id;
END;
CREATE TRIGGER room_proposal_updated AFTER UPDATE ON room_character_proposals BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND room_id=NEW.room_id;
END;
CREATE TRIGGER room_combat_receipt AFTER INSERT ON combat_action_receipts BEGIN
    INSERT INTO room_changes(room_id,kind,payload_json)
    SELECT r.room_id,'combat',json_object('encounter_id',NEW.encounter_id,'revision',json_extract(NEW.projection_json,'$.revision'),'events',json_extract(NEW.projection_json,'$.events'))
    FROM coop_rooms r JOIN combat_encounters c ON c.session_id=r.session_id WHERE r.active=1 AND c.id=NEW.encounter_id;
END;
CREATE TRIGGER room_message_created AFTER INSERT ON messages
WHEN NEW.role IN('user','assistant') AND NOT EXISTS(SELECT 1 FROM room_restore_guard WHERE session_id=NEW.session_id)
BEGIN
    INSERT OR IGNORE INTO shared_messages(message_id,session_id)
    SELECT NEW.id,NEW.session_id WHERE EXISTS(SELECT 1 FROM coop_rooms WHERE active=1 AND session_id=NEW.session_id);
    INSERT INTO room_changes(room_id,kind,payload_json)
    SELECT room_id,'message',json_object('message_id',NEW.id) FROM coop_rooms WHERE active=1 AND session_id=NEW.session_id;
END;
CREATE TRIGGER room_timeline_reset AFTER INSERT ON snapshot_restores BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'timeline_reset' FROM coop_rooms WHERE active=1 AND session_id=NEW.session_id;
END;
CREATE TRIGGER room_message_deleted AFTER DELETE ON messages BEGIN
    INSERT INTO room_changes(room_id,kind)
    SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND session_id=OLD.session_id
    AND EXISTS(SELECT 1 FROM shared_messages WHERE message_id=OLD.id)
    AND NOT EXISTS(SELECT 1 FROM room_restore_guard WHERE session_id=OLD.session_id);
    DELETE FROM shared_messages WHERE message_id=OLD.id;
END;
CREATE TRIGGER room_message_updated AFTER UPDATE ON messages
WHEN NEW.role IN('user','assistant') AND EXISTS(SELECT 1 FROM shared_messages WHERE message_id=NEW.id)
AND NOT EXISTS(SELECT 1 FROM room_restore_guard WHERE session_id=NEW.session_id)
BEGIN
    INSERT INTO room_changes(room_id,kind,payload_json) SELECT room_id,'message',json_object('message_id',NEW.id) FROM coop_rooms WHERE active=1 AND session_id=NEW.session_id;
END;
CREATE TRIGGER room_agent_created AFTER INSERT ON agent_turn_operations BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND session_id=NEW.session_id;
END;
CREATE TRIGGER room_agent_status AFTER UPDATE OF status ON agent_turn_operations WHEN OLD.status!=NEW.status BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND session_id=NEW.session_id;
END;
CREATE TRIGGER room_scene_created AFTER INSERT ON scenes BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_scene_updated AFTER UPDATE ON scenes BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_journal_created AFTER INSERT ON journal_entries BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_journal_updated AFTER UPDATE ON journal_entries BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_npc_created AFTER INSERT ON npc_memory BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_npc_updated AFTER UPDATE ON npc_memory BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=NEW.campaign_id;
END;
CREATE TRIGGER room_scene_deleted AFTER DELETE ON scenes BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=OLD.campaign_id;
END;
CREATE TRIGGER room_journal_deleted AFTER DELETE ON journal_entries BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=OLD.campaign_id;
END;
CREATE TRIGGER room_npc_deleted AFTER DELETE ON npc_memory BEGIN
    INSERT INTO room_changes(room_id,kind) SELECT room_id,'state' FROM coop_rooms WHERE active=1 AND campaign_id=OLD.campaign_id;
END;
