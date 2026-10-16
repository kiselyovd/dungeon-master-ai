-- No backfill: old records cannot prove their original provider origin.
ALTER TABLE agent_turn_operations ADD COLUMN execution_provider TEXT;
ALTER TABLE agent_turn_operations ADD COLUMN execution_model TEXT;
ALTER TABLE room_dm_turns ADD COLUMN execution_provider TEXT;
ALTER TABLE room_dm_turns ADD COLUMN execution_model TEXT;
