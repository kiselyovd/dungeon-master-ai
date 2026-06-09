-- M11-DM: add damage resistance/immunity/vulnerability columns to combat_tokens.
-- JSON arrays of damage-type strings (e.g. '["fire","cold"]').
-- Nullable: NULL means no resistances/immunities/vulnerabilities set.

ALTER TABLE combat_tokens ADD COLUMN resistances     TEXT;  -- JSON array, e.g. '["fire"]'
ALTER TABLE combat_tokens ADD COLUMN immunities      TEXT;  -- JSON array, e.g. '["poison","psychic"]'
ALTER TABLE combat_tokens ADD COLUMN vulnerabilities TEXT;  -- JSON array, e.g. '["bludgeoning"]'
