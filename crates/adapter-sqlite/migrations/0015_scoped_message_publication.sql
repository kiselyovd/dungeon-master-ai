-- Only room-scoped, lease-checked checkpoints publish narration.
-- Ordinary solo MessageRepository writes remain private, even with a durable room.
DROP TRIGGER room_message_created;
CREATE TRIGGER room_agent_message_published AFTER INSERT ON room_agent_messages BEGIN
    INSERT OR IGNORE INTO shared_messages(message_id,session_id)
    SELECT m.id,m.session_id FROM messages m
    JOIN coop_rooms r ON r.room_id=NEW.room_id AND r.session_id=m.session_id
    WHERE m.id=NEW.message_id AND r.active=1 AND m.role IN('user','assistant')
    AND m.tool_calls IS NULL;
    INSERT INTO room_changes(room_id,kind,payload_json)
    SELECT NEW.room_id,'message',json_object('message_id',NEW.message_id)
    WHERE EXISTS(SELECT 1 FROM shared_messages WHERE message_id=NEW.message_id);
END;
