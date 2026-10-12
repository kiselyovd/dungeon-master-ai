-- Capture execution ownership before planning so a manifest-free failure cannot
-- be attributed to a replacement hero. Legacy bindings are backfilled only from
-- their existing protected manifest; unknown ownership remains NULL.
ALTER TABLE agent_turn_operations ADD COLUMN actor_id TEXT;
ALTER TABLE room_dm_turns ADD COLUMN actor_id TEXT;

UPDATE agent_turn_operations
SET actor_id = (
    SELECT json_extract(m.manifest_json, '$.scope.actor_id')
    FROM solo_human_command_manifests m
    WHERE m.request_id = agent_turn_operations.request_id
      AND json_extract(m.manifest_json, '$.scope.request_id') = agent_turn_operations.request_id
      AND json_extract(m.manifest_json, '$.scope.campaign_id') = agent_turn_operations.campaign_id
      AND json_extract(m.manifest_json, '$.scope.session_id') = agent_turn_operations.session_id
);

UPDATE room_dm_turns
SET actor_id = (
    SELECT json_extract(m.manifest_json, '$.scope.actor_id')
    FROM room_human_command_manifests m
    WHERE m.room_id = room_dm_turns.room_id
      AND m.member_id = room_dm_turns.member_id
      AND m.turn_id = room_dm_turns.turn_id
      AND json_extract(m.manifest_json, '$.scope.request_id') = room_dm_turns.turn_id
);
