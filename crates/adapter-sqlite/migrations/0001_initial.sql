-- Development baseline for the SRD 5.2.1 campaign platform.
-- Pre-release databases are intentionally reset rather than upgraded from the legacy schema.

CREATE TABLE campaigns (
    id             TEXT PRIMARY KEY NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    ruleset_id     TEXT NOT NULL DEFAULT 'srd-5.2.1' CHECK (ruleset_id = 'srd-5.2.1'),
    title          TEXT NOT NULL DEFAULT '',
    premise        TEXT NOT NULL DEFAULT '',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL DEFAULT '',
    -- Transitional columns consumed by the compatibility server until Task 3.
    name           TEXT NOT NULL DEFAULT '',
    language       TEXT NOT NULL DEFAULT 'en',
    last_played    TEXT NOT NULL DEFAULT ''
);

CREATE TABLE sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    revision    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT '',
    updated_at  TEXT NOT NULL DEFAULT '',
    -- Transitional columns consumed by the compatibility server until Task 3.
    number      INTEGER NOT NULL DEFAULT 1,
    started_at  TEXT NOT NULL DEFAULT '',
    ended_at    TEXT
);

CREATE INDEX sessions_campaign ON sessions(campaign_id);

CREATE TABLE campaign_events (
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    revision    INTEGER NOT NULL,
    event_id    TEXT NOT NULL UNIQUE,
    kind        TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    causation_id TEXT,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (campaign_id, revision)
);

CREATE TABLE campaign_snapshots (
    id             TEXT PRIMARY KEY NOT NULL,
    campaign_id    TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    session_id     TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    revision       INTEGER NOT NULL,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    ruleset_id     TEXT NOT NULL CHECK (ruleset_id = 'srd-5.2.1'),
    projection_json TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    UNIQUE (campaign_id, revision)
);

CREATE TABLE saves (
    id          TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    snapshot_id TEXT REFERENCES campaign_snapshots(id) ON DELETE SET NULL,
    revision    INTEGER NOT NULL,
    title       TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE snapshots (
    id             TEXT PRIMARY KEY NOT NULL,
    session_id     TEXT NOT NULL,
    turn_number    INTEGER NOT NULL,
    created_at     TEXT NOT NULL,
    game_state     TEXT NOT NULL,
    state_hash     TEXT,
    narration      TEXT,
    player_action  TEXT,
    parent_save_id TEXT,
    branch_id      TEXT,
    kind           TEXT NOT NULL DEFAULT 'auto',
    title          TEXT NOT NULL DEFAULT '',
    summary        TEXT NOT NULL DEFAULT '',
    tag            TEXT NOT NULL DEFAULT 'exploration'
);

CREATE INDEX snapshots_session_turn ON snapshots(session_id, turn_number);
CREATE INDEX snapshots_session_created ON snapshots(session_id, created_at);

CREATE TABLE messages (
    id           TEXT PRIMARY KEY NOT NULL,
    session_id   TEXT NOT NULL,
    role         TEXT NOT NULL,
    parts        TEXT NOT NULL,
    tool_calls   TEXT,
    tool_call_id TEXT,
    is_error     INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL
);

CREATE INDEX messages_session_created ON messages(session_id, created_at);

CREATE TABLE scenes (
    id           TEXT PRIMARY KEY NOT NULL,
    campaign_id  TEXT NOT NULL,
    title        TEXT NOT NULL,
    subtitle     TEXT,
    mode         TEXT NOT NULL,
    image_prompt TEXT,
    created_at   TEXT NOT NULL
);

CREATE INDEX scenes_campaign_created ON scenes(campaign_id, created_at);

CREATE TABLE combat (
    id              TEXT PRIMARY KEY NOT NULL,
    session_id      TEXT NOT NULL,
    revision        INTEGER NOT NULL DEFAULT 0,
    projection_json TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE combat_encounters (
    id          TEXT PRIMARY KEY NOT NULL,
    session_id  TEXT NOT NULL,
    round       INTEGER NOT NULL DEFAULT 1,
    active_turn TEXT,
    started_at  TEXT NOT NULL,
    ended_at    TEXT,
    initiative  TEXT NOT NULL,
    terrain     TEXT,
    revision    INTEGER NOT NULL DEFAULT 0,
    projection  TEXT
);

CREATE TABLE combat_tokens (
    id              TEXT PRIMARY KEY NOT NULL,
    encounter_id    TEXT NOT NULL REFERENCES combat_encounters(id),
    name            TEXT NOT NULL,
    current_hp      INTEGER NOT NULL,
    max_hp          INTEGER NOT NULL,
    ac              INTEGER NOT NULL,
    pos_x           INTEGER NOT NULL,
    pos_y           INTEGER NOT NULL,
    conditions      TEXT NOT NULL DEFAULT '[]',
    is_dead         INTEGER NOT NULL DEFAULT 0,
    resistances     TEXT,
    immunities      TEXT,
    vulnerabilities TEXT
);

CREATE TABLE journal_entries (
    id          TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL,
    chapter     TEXT,
    entry_html  TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX journal_entries_campaign ON journal_entries(campaign_id, created_at);

CREATE TABLE npc_memory (
    id          TEXT PRIMARY KEY NOT NULL,
    campaign_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT '',
    disposition TEXT NOT NULL DEFAULT 'unknown',
    trust       INTEGER NOT NULL DEFAULT 0,
    facts       TEXT NOT NULL DEFAULT '[]',
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX npc_memory_campaign_name ON npc_memory(campaign_id, name);

CREATE TABLE srd_chunks (
    id         TEXT PRIMARY KEY NOT NULL,
    source_key TEXT NOT NULL UNIQUE,
    text_en    TEXT NOT NULL,
    embedding  BLOB
);
