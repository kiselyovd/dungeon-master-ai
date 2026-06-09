//! Tool catalog exposed to the LLM.
//!
//! `all_tools()` returns the full set of tool definitions the agent loop hands
//! to the provider for each turn. Descriptions stay in English per the
//! bilingual discipline (tool names are part of the protocol contract).

use app_llm::Tool;
use serde_json::json;

/// M7.5-DM: classify which subsystem handles a given tool. Surfaced to the
/// frontend Tool Inspector as a pill so the user can see at a glance whether
/// a tool ran inside the rules engine (deterministic) or was delegated to an
/// external provider (e.g. image gen). Returns a stable kebab-case string the
/// frontend maps to a CSS class.
pub fn classify_handler(tool_name: &str) -> &'static str {
    match tool_name {
        "generate_map" | "generate_illustration" => "image-provider",
        _ => "engine",
    }
}

/// Map an image-producing tool name to the frontend routing discriminator:
/// `map` paints the VTT background (left), `chat` renders inline in the
/// tool-call card (right). `None` for non-image tools.
pub fn image_kind(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "generate_map" => Some("map"),
        "generate_illustration" => Some("chat"),
        _ => None,
    }
}

/// Which modality-specific tools to include in the catalog. M7-DM addition:
/// when image generation is disabled in Settings, omit `generate_image` so
/// the LLM doesn't try to call something that will fail.
#[derive(Debug, Clone, Copy, Default)]
pub struct ToolAvailability {
    pub image: bool,
    pub video: bool,
}

impl ToolAvailability {
    pub const fn all() -> Self {
        Self {
            image: true,
            video: true,
        }
    }
}

/// The full set of tools exposed to the LLM.
/// Descriptions are in English per bilingual discipline (tool names stay English).
///
/// Equivalent to `all_tools_with(ToolAvailability::all())`. Existing M3..M6
/// callers stay on this signature.
pub fn all_tools() -> Vec<Tool> {
    all_tools_with(ToolAvailability::all())
}

/// M7-DM: build the tool catalog filtered by which modalities the user has
/// enabled in Settings. Image/video tool definitions are omitted when their
/// modality is disabled.
pub fn all_tools_with(availability: ToolAvailability) -> Vec<Tool> {
    let mut tools = all_tools_core();
    if availability.image {
        tools.push(generate_map_tool());
        tools.push(generate_illustration_tool());
    }
    // generate_video tool definition is added in M7.5-DM once the video tool
    // executor lands. For now, video gen runs out-of-band via the SSE route.
    let _ = availability.video;
    tools
}

/// Tools that only make sense once combat has started. Withholding them on
/// exploration/narration turns shrinks the catalog a small local model (Gemma 4
/// E2B) must reason over from 16 to ~9 - the difference between it reliably
/// calling the right tool and drowning in deliberation. `start_combat` is NOT in
/// this set: the model needs it available to BEGIN combat.
const COMBAT_ONLY_TOOLS: &[&str] = &[
    "apply_damage",
    "apply_healing",
    "end_combat",
    "add_token",
    "update_token",
    "remove_token",
    "cast_spell",
];

/// Select the tools to expose for this turn. Outside combat, the
/// combat-management tools are withheld so the model sees only the
/// exploration-relevant subset. In combat, the full catalog is exposed.
pub fn tools_for_phase(availability: ToolAvailability, in_combat: bool) -> Vec<Tool> {
    let mut tools = all_tools_with(availability);
    if !in_combat {
        tools.retain(|t| !COMBAT_ONLY_TOOLS.contains(&t.name.as_str()));
    }
    tools
}

fn all_tools_core() -> Vec<Tool> {
    vec![
        Tool {
            name: "roll_dice".into(),
            description: "Roll a dice expression and return the result. Always use this tool for any dice roll. Never invent numbers.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "dice": { "type": "string", "description": "Dice expression e.g. 2d6, 1d20, 4d6" },
                    "modifier": { "type": "integer", "description": "Flat bonus/penalty to add to total" },
                    "advantage": { "type": "boolean", "description": "Roll twice, take higher" },
                    "reason": { "type": "string", "description": "Short description of why we are rolling" }
                },
                "required": ["dice"]
            }),
        },
        Tool {
            name: "apply_damage".into(),
            description: "Apply damage to a combatant. Runs through resistance/immunity table. Returns new HP.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "token_id": { "type": "string" },
                    "amount": { "type": "integer", "minimum": 0 },
                    "type": { "type": "string", "enum": ["slashing","piercing","bludgeoning","fire","cold","lightning","thunder","acid","poison","necrotic","radiant","psychic","force"] }
                },
                "required": ["token_id","amount","type"]
            }),
        },
        Tool {
            name: "apply_healing".into(),
            description: "Restore hit points to a combatant. Healing is capped at the token's max HP. Returns new HP.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "token_id": { "type": "string" },
                    "amount": { "type": "integer", "minimum": 0 }
                },
                "required": ["token_id","amount"]
            }),
        },
        Tool {
            name: "start_combat".into(),
            description: "Begin combat. Pass one initiative entry per combatant (the player plus each enemy). ONLY `name` is required - the engine auto-rolls initiative and fills sensible default stats (HP/AC) for anything you omit. Call this immediately when a fight starts; never ask the player for stats or initiative.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "initiative_entries": {
                        "type": "array",
                        "description": "One entry per combatant. Just the name is enough, e.g. [{\"name\":\"Hero\"},{\"name\":\"Skeleton\"}].",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "Combatant name (required)." },
                                "hp": { "type": "integer", "description": "Optional; defaults to a sane value." },
                                "max_hp": { "type": "integer", "description": "Optional; defaults to hp." },
                                "ac": { "type": "integer", "description": "Optional; defaults to 10." },
                                "roll": { "type": "integer", "description": "Optional initiative roll; the engine rolls one if omitted." },
                                "dex_mod": { "type": "integer", "description": "Optional dexterity modifier." }
                            },
                            "required": ["name"]
                        }
                    }
                },
                "required": ["initiative_entries"]
            }),
        },
        Tool {
            name: "end_combat".into(),
            description: "End the active combat encounter. Call when all hostiles are defeated or fled.".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        Tool {
            name: "add_token".into(),
            description: "Add a new token (NPC or monster) to the VTT map.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "name": { "type": "string" },
                    "x": { "type": "integer" },
                    "y": { "type": "integer" },
                    "hp": { "type": "integer" },
                    "max_hp": { "type": "integer" },
                    "ac": { "type": "integer" }
                },
                "required": ["id","name","x","y","hp","max_hp","ac"]
            }),
        },
        Tool {
            name: "update_token".into(),
            description: "Update one or more fields on an existing token (position, HP, conditions).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "x": { "type": "integer" },
                    "y": { "type": "integer" },
                    "hp": { "type": "integer" },
                    "conditions": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["id"]
            }),
        },
        Tool {
            name: "remove_token".into(),
            description: "Remove a token from the VTT map.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }),
        },
        Tool {
            name: "set_scene".into(),
            description: "Change the current scene. Provide a title, subtitle, mode (exploration or combat), and an optional image prompt for scene art generation.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "subtitle": { "type": "string" },
                    "mode": { "type": "string", "enum": ["exploration","combat"] },
                    "image_prompt": { "type": "string" }
                },
                "required": ["title","mode"]
            }),
        },
        Tool {
            name: "cast_spell".into(),
            description: "Cast a spell. The engine validates slot availability, AoE targets, and save DCs.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "caster_id": { "type": "string" },
                    "spell": { "type": "string", "description": "SRD spell key e.g. magic_missile" },
                    "targets": { "type": "array", "items": { "type": "string" }, "description": "token ids" },
                    "slot_level": { "type": "integer", "minimum": 1, "maximum": 9 }
                },
                "required": ["caster_id","spell","targets"]
            }),
        },
        Tool {
            name: "remember_npc".into(),
            description: "Record a new memory fact about an NPC. Use after any significant interaction.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "fact": { "type": "string" },
                    "disposition": { "type": "string", "enum": ["friendly","neutral","hostile","unknown"] },
                    "role": { "type": "string" }
                },
                "required": ["name","fact"]
            }),
        },
        Tool {
            name: "recall_npc".into(),
            description: "Retrieve stored memory facts for an NPC by name.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "journal_append".into(),
            description: "Append an entry to the bard's campaign journal. Write in first-person bard voice, past tense, English or the narration language.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "entry_html": { "type": "string", "description": "HTML prose for the journal entry" },
                    "chapter": { "type": "string", "description": "Chapter heading (optional)" }
                },
                "required": ["entry_html"]
            }),
        },
        Tool {
            name: "quick_save".into(),
            description: "Save the current game state. Linear save - overwrites the current save for this campaign.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string" }
                }
            }),
        },
        Tool {
            name: "query_rules".into(),
            description: "Look up D&D 5e SRD rules relevant to a question. Returns the top matching rule chunks.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "question": { "type": "string" }
                },
                "required": ["question"]
            }),
        },
    ]
}

fn generate_map_tool() -> Tool {
    Tool {
        name: "generate_map".into(),
        description: "Render a TOP-DOWN tactical battle map of the current \
            location and show it on the VTT (the left-hand board). Call this when \
            the party enters a place where positioning matters (a room, dungeon, \
            street, clearing) or when a fight is about to start. Pass a concrete \
            visual prompt describing the location's layout and terrain - NOT a \
            character or a single object. The engine renders it bird's-eye, \
            grid-aligned. At most once per location."
            .into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "Location layout/terrain, e.g. 'ruined throne hall, broken pillars, central dais, rubble'" }
            },
            "required": ["prompt"]
        }),
    }
}

fn generate_illustration_tool() -> Tool {
    Tool {
        name: "generate_illustration".into(),
        description: "Show the players a cinematic illustration in the chat (the \
            right-hand panel): a character, creature, item, or a dramatic view of a \
            scene. Call this when the player asks to see/draw/show someone or \
            something, or to punctuate a dramatic moment. This is NOT a map - it \
            does not change the VTT board. Pass a concrete visual prompt (~30 words)."
            .into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "30-word content description of the subject/scene" },
                "style": { "type": "string", "enum": ["dark_fantasy","portrait"] }
            },
            "required": ["prompt"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_default_includes_image_tools() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.name == "generate_map"));
        assert!(tools.iter().any(|t| t.name == "generate_illustration"));
    }

    #[test]
    fn all_tools_with_image_disabled_omits_image_tools() {
        let tools = all_tools_with(ToolAvailability {
            image: false,
            video: false,
        });
        assert!(!tools.iter().any(|t| t.name == "generate_map"));
        assert!(!tools.iter().any(|t| t.name == "generate_illustration"));
    }

    #[test]
    fn classify_handler_routes_image_tools_to_image_provider() {
        assert_eq!(classify_handler("generate_map"), "image-provider");
        assert_eq!(classify_handler("generate_illustration"), "image-provider");
    }

    #[test]
    fn image_kind_maps_tool_names() {
        assert_eq!(image_kind("generate_map"), Some("map"));
        assert_eq!(image_kind("generate_illustration"), Some("chat"));
        assert_eq!(image_kind("roll_dice"), None);
    }

    #[test]
    fn tools_for_phase_withholds_combat_tools_outside_combat() {
        let tools = tools_for_phase(ToolAvailability::all(), false);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        for combat in COMBAT_ONLY_TOOLS {
            assert!(
                !names.contains(combat),
                "{combat} must be hidden outside combat"
            );
        }
        // Exploration tools and start_combat stay available.
        assert!(names.contains(&"roll_dice"));
        assert!(names.contains(&"start_combat"));
        assert!(names.contains(&"generate_map"));
        assert!(names.contains(&"generate_illustration"));
        assert!(names.contains(&"set_scene"));
    }

    #[test]
    fn tools_for_phase_exposes_full_catalog_in_combat() {
        let in_combat = tools_for_phase(ToolAvailability::all(), true);
        let full = all_tools_with(ToolAvailability::all());
        assert_eq!(in_combat.len(), full.len());
        let names: Vec<&str> = in_combat.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"apply_damage"));
        assert!(names.contains(&"cast_spell"));
    }

    #[test]
    fn classify_handler_routes_engine_tools_to_engine() {
        for name in ["roll_dice", "apply_damage", "set_scene", "query_rules"] {
            assert_eq!(classify_handler(name), "engine");
        }
    }

    #[test]
    fn classify_handler_treats_unknown_tools_as_engine() {
        assert_eq!(classify_handler("future-tool-x"), "engine");
    }

    #[test]
    fn all_tools_always_includes_core_tools() {
        let tools = all_tools_with(ToolAvailability {
            image: false,
            video: false,
        });
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"roll_dice"));
        assert!(names.contains(&"apply_damage"));
        assert!(names.contains(&"query_rules"));
    }

    #[test]
    fn all_tools_includes_apply_healing() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.name == "apply_healing"));
    }
}
