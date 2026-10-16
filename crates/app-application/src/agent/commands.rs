use app_domain::combat::types::DamageType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollDiceCommand {
    pub dice: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advantage: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyDamageCommand {
    pub token_id: String,
    pub amount: i64,
    #[serde(rename = "type")]
    pub damage_type: DamageType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyHealingCommand {
    pub token_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitiativeEntryCommand {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dex_mod: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartCombatCommand {
    pub initiative_entries: Vec<InitiativeEntryCommand>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndCombatCommand {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddTokenCommand {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<DamageType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub immunities: Vec<DamageType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vulnerabilities: Vec<DamageType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateTokenCommand {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resistances: Option<Vec<DamageType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immunities: Option<Vec<DamageType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vulnerabilities: Option<Vec<DamageType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoveTokenCommand {
    pub id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneMode {
    Exploration,
    Combat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetSceneCommand {
    pub title: String,
    pub mode: SceneMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CastSpellCommand {
    pub caster_id: String,
    pub spell: String,
    pub targets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_level: Option<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NpcDisposition {
    Friendly,
    Neutral,
    Hostile,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberNpcCommand {
    pub name: String,
    pub fact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<NpcDisposition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecallNpcCommand {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalAppendCommand {
    pub entry_html: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickSaveCommand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryRulesCommand {
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerateMapCommand {
    pub prompt: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IllustrationStyle {
    DarkFantasy,
    Portrait,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerateIllustrationCommand {
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<IllustrationStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerateVideoCommand {
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentToolCommand {
    RollDice(RollDiceCommand),
    ApplyDamage(ApplyDamageCommand),
    ApplyHealing(ApplyHealingCommand),
    StartCombat(StartCombatCommand),
    EndCombat(EndCombatCommand),
    AddToken(AddTokenCommand),
    UpdateToken(UpdateTokenCommand),
    RemoveToken(RemoveTokenCommand),
    SetScene(SetSceneCommand),
    CastSpell(CastSpellCommand),
    RememberNpc(RememberNpcCommand),
    RecallNpc(RecallNpcCommand),
    JournalAppend(JournalAppendCommand),
    QuickSave(QuickSaveCommand),
    QueryRules(QueryRulesCommand),
    GenerateMap(GenerateMapCommand),
    GenerateIllustration(GenerateIllustrationCommand),
    GenerateVideo(GenerateVideoCommand),
}

impl AgentToolCommand {
    pub const fn tool_name(&self) -> &'static str {
        match self {
            Self::RollDice(_) => "roll_dice",
            Self::ApplyDamage(_) => "apply_damage",
            Self::ApplyHealing(_) => "apply_healing",
            Self::StartCombat(_) => "start_combat",
            Self::EndCombat(_) => "end_combat",
            Self::AddToken(_) => "add_token",
            Self::UpdateToken(_) => "update_token",
            Self::RemoveToken(_) => "remove_token",
            Self::SetScene(_) => "set_scene",
            Self::CastSpell(_) => "cast_spell",
            Self::RememberNpc(_) => "remember_npc",
            Self::RecallNpc(_) => "recall_npc",
            Self::JournalAppend(_) => "journal_append",
            Self::QuickSave(_) => "quick_save",
            Self::QueryRules(_) => "query_rules",
            Self::GenerateMap(_) => "generate_map",
            Self::GenerateIllustration(_) => "generate_illustration",
            Self::GenerateVideo(_) => "generate_video",
        }
    }

    pub fn to_args(&self) -> Value {
        match self {
            Self::RollDice(value) => serde_json::to_value(value),
            Self::ApplyDamage(value) => serde_json::to_value(value),
            Self::ApplyHealing(value) => serde_json::to_value(value),
            Self::StartCombat(value) => serde_json::to_value(value),
            Self::EndCombat(value) => serde_json::to_value(value),
            Self::AddToken(value) => serde_json::to_value(value),
            Self::UpdateToken(value) => serde_json::to_value(value),
            Self::RemoveToken(value) => serde_json::to_value(value),
            Self::SetScene(value) => serde_json::to_value(value),
            Self::CastSpell(value) => serde_json::to_value(value),
            Self::RememberNpc(value) => serde_json::to_value(value),
            Self::RecallNpc(value) => serde_json::to_value(value),
            Self::JournalAppend(value) => serde_json::to_value(value),
            Self::QuickSave(value) => serde_json::to_value(value),
            Self::QueryRules(value) => serde_json::to_value(value),
            Self::GenerateMap(value) => serde_json::to_value(value),
            Self::GenerateIllustration(value) => serde_json::to_value(value),
            Self::GenerateVideo(value) => serde_json::to_value(value),
        }
        .expect("typed tool command serialization is infallible")
    }
}
