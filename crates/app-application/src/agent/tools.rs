use crate::models::chat::Tool;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default)]
pub struct AgentToolCatalog {
    pub exploration: Vec<Tool>,
    pub combat: Vec<Tool>,
}

impl AgentToolCatalog {
    pub fn for_context(&self, combat_active: bool, adventure_active: bool) -> Vec<Tool> {
        let mut tools = self.for_combat_state(combat_active);
        tools.retain(|tool| {
            if adventure_active {
                authored_tool_allowed(&tool.name)
            } else {
                tool.name != "adventure_action"
            }
        });
        tools
    }
    pub fn for_combat_state(&self, combat_active: bool) -> Vec<Tool> {
        if combat_active {
            self.combat.clone()
        } else {
            self.exploration.clone()
        }
    }
}

pub fn authored_tool_allowed(name: &str) -> bool {
    matches!(
        name,
        "adventure_action"
            | "combat_action"
            | "rest_action"
            | "roll_dice"
            | "query_rules"
            | "quick_save"
            | "generate_map"
            | "generate_illustration"
            | "generate_video"
    )
}

pub fn classify_handler(tool_name: &str) -> &'static str {
    match tool_name {
        "generate_map" | "generate_illustration" => "image-provider",
        "generate_video" => "video-provider",
        _ => "engine",
    }
}

pub fn image_kind(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "generate_map" => Some("map"),
        "generate_illustration" => Some("chat"),
        _ => None,
    }
}

pub fn video_kind(tool_name: &str) -> Option<&'static str> {
    (tool_name == "generate_video").then_some("chat")
}

pub fn is_media_tool(tool_name: &str) -> bool {
    image_kind(tool_name).is_some() || video_kind(tool_name).is_some()
}
