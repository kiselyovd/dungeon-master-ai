//! Tool catalog exposed to the LLM.
//!
//! `all_tools()` returns the full set of tool definitions the agent loop hands
//! to the provider for each turn. Descriptions stay in English per the
//! bilingual discipline (tool names are part of the protocol contract).

use app_llm::Tool;
use serde_json::json;

/// The full set of tools exposed to the LLM.
/// Descriptions are in English per bilingual discipline (tool names stay English).
pub fn all_tools() -> Vec<Tool> {
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
            name: "start_combat".into(),
            description: "Transition the scene to combat mode. Provide initiative entries for all combatants.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "initiative_entries": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "name": { "type": "string" },
                                "roll": { "type": "integer" },
                                "dex_mod": { "type": "integer" },
                                "hp": { "type": "integer" },
                                "max_hp": { "type": "integer" },
                                "ac": { "type": "integer" }
                            },
                            "required": ["id","name","roll","dex_mod","hp","max_hp","ac"]
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
            name: "generate_image".into(),
            description: "Generate a scene illustration. Rate limited: call at most once per scene change.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "prompt": { "type": "string", "description": "30-word content description" },
                    "style": { "type": "string", "enum": ["dark_fantasy","portrait","map"] }
                },
                "required": ["prompt"]
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
