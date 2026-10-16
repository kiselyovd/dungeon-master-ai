CREATE TABLE campaign_characters (
    campaign_id TEXT PRIMARY KEY NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    character_id TEXT NOT NULL,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    ruleset_id TEXT NOT NULL CHECK (ruleset_id = 'srd-5.2.1'),
    character_json TEXT NOT NULL CHECK (json_valid(character_json)),
    updated_at TEXT NOT NULL
);

CREATE TABLE character_commits (
    campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    causation_id TEXT NOT NULL,
    request_json TEXT NOT NULL,
    state_json TEXT NOT NULL,
    PRIMARY KEY (campaign_id, causation_id)
);

-- Backfill only structurally complete sheets. Keep incomplete historical snapshots
-- intact for recovery instead of turning them into an unreadable current character.
WITH source_snapshots AS (
    SELECT cs.*, CASE WHEN json_valid(projection_json) THEN projection_json ELSE '{}' END AS safe_projection
    FROM campaign_snapshots cs
), eligible_snapshots AS (
    SELECT * FROM source_snapshots
    WHERE json_type(safe_projection, '$.character') = 'object'
      AND json_type(safe_projection, '$.character.name') = 'text'
      AND length(trim(json_extract(safe_projection, '$.character.name'), char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) > 0
      AND json_type(safe_projection, '$.character.heroClass') = 'text'
      AND length(trim(json_extract(safe_projection, '$.character.heroClass'), char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) > 0
      AND json_type(safe_projection, '$.character.level') = 'integer'
      AND json_extract(safe_projection, '$.character.level') BETWEEN 1 AND 20
      AND json_type(safe_projection, '$.character.hpMax') = 'integer'
      AND json_extract(safe_projection, '$.character.hpMax') BETWEEN 1 AND 10000
      AND json_type(safe_projection, '$.character.hp') = 'integer'
      AND json_extract(safe_projection, '$.character.hp') BETWEEN 0 AND json_extract(safe_projection, '$.character.hpMax')
      AND json_type(safe_projection, '$.character.ac') = 'integer'
      AND json_extract(safe_projection, '$.character.ac') BETWEEN 1 AND 40
      AND json_type(safe_projection, '$.character.inventory') = 'array'
      AND json_type(safe_projection, '$.character.abilities.str') = 'integer'
      AND json_extract(safe_projection, '$.character.abilities.str') BETWEEN 1 AND 30
      AND json_type(safe_projection, '$.character.abilities.dex') = 'integer'
      AND json_extract(safe_projection, '$.character.abilities.dex') BETWEEN 1 AND 30
      AND json_type(safe_projection, '$.character.abilities.con') = 'integer'
      AND json_extract(safe_projection, '$.character.abilities.con') BETWEEN 1 AND 30
      AND json_type(safe_projection, '$.character.abilities.int') = 'integer'
      AND json_extract(safe_projection, '$.character.abilities.int') BETWEEN 1 AND 30
      AND json_type(safe_projection, '$.character.abilities.wis') = 'integer'
      AND json_extract(safe_projection, '$.character.abilities.wis') BETWEEN 1 AND 30
      AND json_type(safe_projection, '$.character.abilities.cha') = 'integer'
      AND json_extract(safe_projection, '$.character.abilities.cha') BETWEEN 1 AND 30
      AND NOT EXISTS (
          SELECT 1 FROM json_each(safe_projection, '$.character.inventory') item
          WHERE item.type IS NOT 'object'
             OR json_type(CASE WHEN item.type = 'object' THEN item.value ELSE '{}' END, '$.id') IS NOT 'text'
             OR length(json_extract(CASE WHEN item.type = 'object' THEN item.value ELSE '{}' END, '$.id')) = 0
             OR json_type(CASE WHEN item.type = 'object' THEN item.value ELSE '{}' END, '$.name') IS NOT 'text'
             OR length(json_extract(CASE WHEN item.type = 'object' THEN item.value ELSE '{}' END, '$.name')) = 0
             OR json_type(CASE WHEN item.type = 'object' THEN item.value ELSE '{}' END, '$.count') IS NOT 'integer'
             OR json_extract(CASE WHEN item.type = 'object' THEN item.value ELSE '{}' END, '$.count') < 0
      )
)
INSERT INTO campaign_characters
    (campaign_id, character_id, schema_version, ruleset_id, character_json, updated_at)
SELECT c.id, cs.id, cs.schema_version, cs.ruleset_id,
       json_extract(cs.safe_projection, '$.character'), cs.created_at
FROM campaigns c
JOIN sessions s ON s.id = (
    SELECT latest.id FROM sessions latest WHERE latest.campaign_id = c.id
    ORDER BY CAST(latest.created_at AS INTEGER) DESC, latest.id DESC LIMIT 1
)
JOIN eligible_snapshots cs ON cs.id = COALESCE(s.active_snapshot_id, (
    SELECT latest.id FROM eligible_snapshots latest WHERE latest.session_id = s.id
    ORDER BY latest.revision DESC, latest.created_at DESC, latest.id DESC LIMIT 1
));
