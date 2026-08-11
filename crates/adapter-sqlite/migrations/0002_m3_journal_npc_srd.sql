-- M3 tables: journal, NPC memory, SRD chunks (for embedding cache).

CREATE TABLE IF NOT EXISTS journal_entries (
    id          TEXT PRIMARY KEY NOT NULL, -- UUID v4
    campaign_id TEXT NOT NULL,
    chapter     TEXT,
    entry_html  TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS journal_entries_campaign
    ON journal_entries(campaign_id, created_at);

CREATE TABLE IF NOT EXISTS npc_memory (
    id          TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT '',
    disposition TEXT NOT NULL DEFAULT 'unknown', -- friendly|neutral|hostile|unknown
    trust       INTEGER NOT NULL DEFAULT 0,      -- -100 to 100
    facts       TEXT NOT NULL DEFAULT '[]',      -- JSON array of {text, created_at}
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS npc_memory_campaign_name
    ON npc_memory(campaign_id, name);

CREATE TABLE IF NOT EXISTS srd_chunks (
    id          TEXT PRIMARY KEY NOT NULL,
    source_key  TEXT NOT NULL UNIQUE,
    text_en     TEXT NOT NULL,
    embedding   BLOB            -- f32 LE bytes, 384 dims for BGE-small-en
);
