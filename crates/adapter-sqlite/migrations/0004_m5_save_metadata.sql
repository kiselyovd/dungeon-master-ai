-- M5 P2.13: extend snapshots table with user-driven save metadata.
--
-- The existing `snapshots` table already stores game_state JSON envelopes
-- written by the agent's `quick_save` tool (see tool_executor.rs). This
-- migration adds the columns the Saves UI needs to render the
-- "Chronicles of Adventure" tome modal: a save kind (auto/manual/checkpoint),
-- a human-readable title + summary, and a tag for the per-row icon.
--
-- All four columns are NOT NULL with sensible defaults so the existing
-- agent-tool insert path (which doesn't supply them) keeps working
-- unchanged. Linear save model only - branching ships in v2 via the
-- existing `parent_save_id` / `branch_id` stub columns from 0001.

ALTER TABLE snapshots ADD COLUMN kind TEXT NOT NULL DEFAULT 'auto';
ALTER TABLE snapshots ADD COLUMN title TEXT NOT NULL DEFAULT '';
ALTER TABLE snapshots ADD COLUMN summary TEXT NOT NULL DEFAULT '';
ALTER TABLE snapshots ADD COLUMN tag TEXT NOT NULL DEFAULT 'exploration';

CREATE INDEX IF NOT EXISTS snapshots_session_created
    ON snapshots(session_id, created_at);
