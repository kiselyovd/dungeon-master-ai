-- M6-DM: scenes table. Stores each set_scene call so the DM can query the
-- current scene across turns and context_builder can inject it into the prompt.

CREATE TABLE IF NOT EXISTS scenes (
    id           TEXT PRIMARY KEY NOT NULL,  -- UUID v4
    campaign_id  TEXT NOT NULL,
    title        TEXT NOT NULL,
    subtitle     TEXT,
    mode         TEXT NOT NULL,              -- exploration | combat | social | rest
    image_prompt TEXT,
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS scenes_campaign_created
    ON scenes(campaign_id, created_at);
