-- M2 initial schema for dungeon-master-ai
-- JSON BLOB snapshots with schema_version for forward compat.
-- parent_save_id and branch_id are v2 forward-compat stubs (always NULL in v1).

CREATE TABLE IF NOT EXISTS campaigns (
    id          TEXT PRIMARY KEY NOT NULL, -- UUID v4
    name        TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'en',
    created_at  TEXT NOT NULL,
    last_played TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id),
    number      INTEGER NOT NULL,
    started_at  TEXT NOT NULL,
    ended_at    TEXT
);

CREATE TABLE IF NOT EXISTS snapshots (
    id             TEXT PRIMARY KEY NOT NULL,
    session_id     TEXT NOT NULL,
    turn_number    INTEGER NOT NULL,
    created_at     TEXT NOT NULL,
    game_state     TEXT NOT NULL, -- JSON: {"schema_version": N, "state": {...}}
    state_hash     TEXT,
    narration      TEXT,          -- JSON: {"en": "...", "ru": "..."}
    player_action  TEXT,          -- JSON
    -- v2 forward-compat stubs
    parent_save_id TEXT,
    branch_id      TEXT
);

CREATE INDEX IF NOT EXISTS snapshots_session_turn
    ON snapshots(session_id, turn_number);

CREATE TABLE IF NOT EXISTS combat_encounters (
    id          TEXT PRIMARY KEY NOT NULL,
    session_id  TEXT NOT NULL,
    round       INTEGER NOT NULL DEFAULT 1,
    active_turn TEXT,             -- combatant UUID currently acting
    started_at  TEXT NOT NULL,
    ended_at    TEXT,
    initiative  TEXT NOT NULL,    -- JSON: sorted InitiativeOrder
    terrain     TEXT              -- JSON: walls, cover
);

CREATE TABLE IF NOT EXISTS combat_tokens (
    id              TEXT PRIMARY KEY NOT NULL,
    encounter_id    TEXT NOT NULL REFERENCES combat_encounters(id),
    name            TEXT NOT NULL,
    current_hp      INTEGER NOT NULL,
    max_hp          INTEGER NOT NULL,
    ac              INTEGER NOT NULL,
    pos_x           INTEGER NOT NULL,
    pos_y           INTEGER NOT NULL,
    conditions      TEXT NOT NULL DEFAULT '[]',  -- JSON array
    is_dead         INTEGER NOT NULL DEFAULT 0
);
