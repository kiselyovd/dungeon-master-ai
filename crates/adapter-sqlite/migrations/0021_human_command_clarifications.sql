ALTER TABLE solo_human_command_manifests ADD COLUMN clarifications_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(clarifications_json));
ALTER TABLE room_human_command_manifests ADD COLUMN clarifications_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(clarifications_json));
ALTER TABLE solo_human_command_manifests ADD COLUMN final_narration_json TEXT CHECK(final_narration_json IS NULL OR json_valid(final_narration_json));
ALTER TABLE room_human_command_manifests ADD COLUMN final_narration_json TEXT CHECK(final_narration_json IS NULL OR json_valid(final_narration_json));
ALTER TABLE agent_turn_messages ADD COLUMN bound_message_id TEXT;
CREATE UNIQUE INDEX agent_turn_bound_message ON agent_turn_messages(request_id,bound_message_id) WHERE bound_message_id IS NOT NULL;
