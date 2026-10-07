//! DM-only facts: never include this string in public state or player events.
use app_domain::adventure::{AdventureState, Location};

pub fn bell_dm_context(state: &AdventureState) -> String {
    let relevant = match state.location {
        Location::Town => "Mila is Tikhon's adult daughter; she knows his last ferry route, not the culprit. Agata saw porters with a wrapped metal object, but cannot identify it. Ten gold pieces are granted by the engine once for rescue; never narrate an additional reward.",
        Location::Quay => "The ward is damaged because the clapper was cut loose recently. Tool marks are visible. The maintained upper path reaches the archive safely; a failed check never blocks every rescue route.",
        Location::Shrine => "Leon knows the ward and holds a working memorial roster copy; he does not know where the clapper is. Either this copy or the drowned original works. He explains the ritual without a compulsory roll.",
        Location::Warehouse => "Varden cut the clapper loose to sell its silver fitting and hid it here. He also illegally obtained the quay. He can return the clapper through negotiation or public intervention without confessing the separate land fraud. He wrongly believes the haunting can be bought off. Do not reveal either secret as already known unless supported by discovered facts/action outcomes.",
        Location::Archive => "Tikhon is alive in the upper chamber and is NOT Savely. He can be escorted along the dry upper route. The memorial roster and later land ledger are separate records. The ledger is optional and never required for rescue or repairing the ward.",
        Location::Belfry => "Savely died forty years ago rescuing the last evacuation boat; the echo repeats that rescue, not modern politics. No historical survivors remain trapped. Repair needs the recovered/repaired clapper, either roster and acknowledgment. Sealing the lower sluice protects the town but closes the quay and does not repair the ward. Combat alone never restores it.",
    };
    let public_state = serde_json::to_string(state).expect("typed adventure state serializes");
    let actions = serde_json::to_string(&state.available_actions())
        .expect("typed adventure actions serialize");
    format!("\n\nAUTHORED ADVENTURE - THE BELL BENEATH THE WATER\nImmutable canon: Veresk is in the Reed March. Historic Savely and current Tikhon are distinct people. Preserve roster/ledger and ward-cause/flood-consequence distinctions. Invent only nonessential flavor. Do not invent quest items, grant resources, change scene or declare an ending through prose. Engine-confirmed actions/facts below are authoritative. Use adventure_action with an action from the JSON list to confirm the player's intention, then narrate its real result. A successful ToolResult confirms a completed action. Narrate that completed outcome even when the refreshed available-action list no longer offers it; that list governs future choices, not the action just completed. Do not execute an additional intention the player did not choose. API identifiers belong only in tool arguments; translate presentation nouns into the narration language and never print raw identifiers in player prose. Tools withheld by the prepared adventure must not be called or simulated in prose. Unsupported actions require an honest explanation and available alternatives. DM-only knowledge is not automatically player knowledge.\nCurrent public state JSON: {public_state}\nAvailable intentions JSON: {actions}\nRelevant NPC knowledge and canon: {relevant}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_is_scene_scoped_and_distinguishes_the_two_ferrymen() {
        let mut state = AdventureState::new();
        assert!(!bell_dm_context(&state).contains("sell its silver fitting"));
        state.location = Location::Archive;
        let context = bell_dm_context(&state);
        assert!(context.contains("NOT Savely"));
        assert!(context.contains("separate records"));
        assert!(!context.contains("sell its silver fitting"));
    }
}
