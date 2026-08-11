-- Authoritative combat projection and optimistic-concurrency revision.
ALTER TABLE combat_encounters ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE combat_encounters ADD COLUMN projection TEXT;
