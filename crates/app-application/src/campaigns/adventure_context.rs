//! Shared discovered narration facts and separate private DM context.
use app_domain::adventure::{AdventureState, Ending, Fact, Location};

const EXAMINED_WARD: &str = "At that inspection, fresh tool marks were found ON THE WARD (оберег), where the bell clapper (язык колокола) had been cut off and removed. The tool marks are on the ward itself, not on a wall or railings. The inspection did not locate the removed clapper. / При осмотре обнаружено: оберег повреждён; на самом обереге свежие следы инструмента в месте, где срезали и сняли язык колокола. Следы не на стене и не на перилах. Где снятый язык находится, осмотр не установил. This historical discovery does not override later confirmed recovery or restoration.";
const QUAY_UPPER_PATH: &str = "The maintained upper path reaches the archive safely; a failed check never blocks every rescue route. / Верхняя тропа ведёт к архиву безопасно; неудачная проверка не перекрывает все пути спасения.";

/// Discovered facts only. This must never include scene-scoped DM secrets.
pub fn bell_public_narration_facts(
    state: &AdventureState,
    inspection_in_this_request: bool,
) -> Vec<crate::agent::narration::NarrationFact> {
    use crate::agent::narration::NarrationFact;
    if state.package_id != app_domain::adventure::BELL_ADVENTURE_ID {
        return Vec::new();
    }
    let mut facts = Vec::new();
    let mut add = |id: &str, text: &str, required| {
        facts.push(NarrationFact {
            id: id.into(),
            text: text.into(),
            required,
        });
    };
    if state.facts.contains(&Fact::WardDamaged) {
        add("ward_inspection", EXAMINED_WARD, inspection_in_this_request);
    }
    if state.location == Location::Quay {
        add("quay_route", QUAY_UPPER_PATH, false);
    }
    if state.facts.contains(&Fact::TikhonRescued) {
        add("tikhon_rescued", "Tikhon is safely at home with his adult daughter Mila in Veresk. He is not still waiting in the archive or on a boat.", false);
    } else if state.facts.contains(&Fact::TikhonLocated) {
        add(
            "tikhon_located",
            "Tikhon has been found alive in the upper chamber. He has not yet been escorted home.",
            false,
        );
    }
    if state.facts.contains(&Fact::RewardGranted) {
        add("rescue_reward", "Exactly ten gold were already granted once for rescue. No additional reward is confirmed.", false);
    }
    if state.facts.contains(&Fact::RosterAvailable) || state.facts.contains(&Fact::EchoAcknowledged)
    {
        add("historic_echo", "Savely died forty years ago saving the last evacuation boat. He and the living ferryman Tikhon are distinct people. The memorial roll is a record, not a grove or genealogy.", false);
    }
    if state.location == Location::Archive {
        add("archive_route", "The upper path and staircase are dry and provide a rescue route. The memorial roll and optional land ledger are separate records.", false);
    }
    match state.ending {
        Some(Ending::ClosedCrossing) => add("ending", "The town is protected by the sealed lower sluice. The ferry route is closed. The ward remains damaged; sealing the gates did not banish or silence the echo.", true),
        Some(Ending::RememberedRiver) => add("ending", "The ward has been restored and the ferry can reopen.", true),
        Some(Ending::UnfinishedDebt) => add("ending", "No successful ending was achieved; unresolved rescue or river danger remains.", true),
        None => {},
    }
    facts
}

pub fn bell_dm_context(state: &AdventureState) -> String {
    let rescued = state.facts.contains(&Fact::TikhonRescued);
    let located = state.facts.contains(&Fact::TikhonLocated);
    let historic_known = state.facts.contains(&Fact::RosterAvailable)
        || state.facts.contains(&Fact::EchoAcknowledged);
    let tikhon = if rescued {
        "Tikhon is safely at home with Mila in Veresk. / Тихон благополучно дома с Милой в Вереске."
    } else if located {
        "Tikhon has been found alive in the upper chamber. / Тихон найден живым в верхней комнате."
    } else {
        "Tikhon has not been found yet. Describe clues, not a confirmed discovery or rescue. / Тихон ещё не найден: описывай следы, не объявляй находку или спасение."
    };
    let reward = if state.facts.contains(&Fact::RewardGranted) {
        "Exactly ten gold were already granted once for rescue. Do not grant any more. / За спасение уже выданы ровно десять золотых; дополнительной награды нет."
    } else {
        "No rescue reward has been granted yet. / Награда за спасение ещё не выдана."
    };
    let historic = if historic_known {
        "The memorial records/acknowledged echo establish that Savely / Савелий died forty years ago saving the last evacuation boat."
    } else {
        "The old echo's identity/history has not been discovered. Describe an old rescue call without a name, date or biography."
    };
    let ending_brief = match state.ending {
        Some(Ending::ClosedCrossing) => "CONFIRMED ENDING: The town is protected by the sealed lower sluice. The trading ferry route is CLOSED and unavailable. The magical ward is still damaged. / Подтверждённый итог: город защищён закрытым нижним шлюзом. Переправа закрыта, торговый путь недоступен. Оберег остаётся повреждённым.",
        Some(Ending::RememberedRiver) => "CONFIRMED ENDING: The ward has been restored and the ferry can reopen. / Подтверждённый итог: оберег восстановлен, переправу можно открыть.",
        Some(Ending::UnfinishedDebt) => "No successful ending was achieved; unresolved rescue or river danger remains. / Спасение или опасность реки ещё не разрешены, победа не объявлена.",
        None => "No ending has been confirmed yet. / Концовка ещё не подтверждена.",
    };
    let ward = if state.facts.contains(&Fact::WardRestored) {
        "The ward is restored. / Оберег восстановлен."
    } else if state.closed
        || state.facts.contains(&Fact::WardDamaged)
        || state.facts.contains(&Fact::WardUnderstood)
        || state.facts.contains(&Fact::LowerSluiceSealed)
    {
        "The ward remains damaged. The echo has not been banished or silenced by closing the sluice. / Оберег повреждён. Закрытие шлюза не изгнало и не заставило замолчать отголосок."
    } else {
        "The ward has not been examined by the hero yet. Describe visible surroundings, not a confirmed cause. / Герой ещё не исследовал оберег: описывай окружение, не объявляй причину."
    };
    let turn_style = if state.closed {
        "This session is already closed. Give its confirmed epilogue, not a new choice. If a quick_save ToolResult is already present, the save is complete: do not call it again. / Сессия уже завершена: расскажи подтверждённый итог, не предлагай новый шаг; подтверждённый сейв не повторяй."
    } else {
        "Describe NPC/environment reactions and the player's already declared action. Leave every new gesture, plan and emotion of the hero to the human player. Decorative details must not become new investigative clues, objects to collect or side quests. / Описывай мир и реакцию NPC; новые жесты, планы и чувства героя выбирает игрок. Не добавляй новые улики, добычу или побочные задания."
    };
    let relevant = match state.location {
        Location::Town => "Mila is Tikhon's adult daughter; she knows his last ferry route, not the culprit. Agata saw porters with a wrapped metal object, but cannot identify it. Ten gold pieces are granted by the engine once for rescue; never narrate an additional reward.",
        Location::Quay if state.facts.contains(&Fact::WardDamaged) => EXAMINED_WARD,
        Location::Quay => "Describe wet boards, the river and the maintained upper path to the archive. The ward has not yet been inspected; do not announce its cause before that discovery.",
        Location::Shrine => "Leon knows the ward and holds a working memorial roster copy; he does not know where the clapper is. Either this copy or the drowned original works. He explains the ritual without a compulsory roll.",
        Location::Warehouse => "Varden cut the clapper loose to sell its silver fitting and hid it here. He also illegally obtained the quay. He can return the clapper through negotiation or public intervention without confessing the separate land fraud. He wrongly believes the haunting can be bought off. Do not reveal either secret as already known unless supported by discovered facts/action outcomes.",
        Location::Archive if rescued => "The rescued ferryman is home with his daughter. The hero remains in the archive. The memorial roll is a record, not a grove or genealogy. The separate land ledger is optional evidence of land ownership.",
        Location::Archive if located => "The living ferryman can now be escorted home along the dry upper route. The memorial roll and land ledger are separate records, not a grove or genealogy; the land ledger is optional.",
        Location::Archive => "The upper path and staircase are dry. The missing ferryman has not yet been found; a voice/clue from the upper room may invite investigation. The memorial roll and land ledger are separate records; the ledger is optional.",
        Location::Belfry => "The echo repeats an old rescue call, not modern politics. No historical survivors remain trapped. Repair needs the recovered/repaired clapper, either roster and acknowledgment. Sealing the lower sluice means closing its existing gates with their built-in mechanism, requiring no new equipment, welding or spell. It protects the town but closes the quay and does not repair the ward. Combat alone never restores it.",
    };
    let route = if state.location == Location::Quay {
        QUAY_UPPER_PATH
    } else {
        ""
    };
    let public_state = serde_json::to_string(state).expect("typed adventure state serializes");
    let actions = serde_json::to_string(&state.available_actions())
        .expect("typed adventure actions serialize");
    format!("\n\nAUTHORED ADVENTURE - THE BELL BENEATH THE WATER\nImmutable canon: Veresk is in the Reed March. The missing living ferryman and the historic echo are distinct people. Preserve roster/ledger and ward-cause/flood-consequence distinctions. Invent only nonessential flavor. Do not invent quest items, grant resources, change scene or declare an ending through prose. Engine-confirmed actions/facts below are authoritative. Use adventure_action with an action from the JSON list to confirm the player's intention, then narrate its real result. A successful ToolResult confirms a completed action. Narrate that completed outcome even when the refreshed available-action list no longer offers it; that list governs future choices, not the action just completed. Do not execute an additional intention the player did not choose. API identifiers belong only in tool arguments; translate presentation nouns into the narration language and never print raw identifiers in player prose. Tools withheld by the prepared adventure must not be called or simulated in prose. Unsupported actions require an honest explanation and available alternatives. DM-only knowledge is not automatically player knowledge. Keep narration to a few clear sentences and optional short NPC dialogue, then one open question only while the session is open. A closed session ends with its confirmed epilogue. Do not print a menu of API actions: the UI presents available choices. Never invent new quest objects or family ties.\nConfirmed narration brief: {tikhon} {reward} {historic} {ending_brief} {ward} {turn_style}\nPresentation glossary EN/RU: Veresk=Вереск; old quay=старый причал; drowned archive=затопленный архив; lower belfry=нижняя звонница; memorial roll=памятный список спасённых; land ledger=земельная книга; Tikhon=Тихон; Mila=Мила; Agata=Агата; Leon=Леон; Varden=Варден. recover_roster means retrieve the original memorial roll, NOT restore a grove. recover_ledger means retrieve land-ownership evidence, NOT farming records.\nCurrent public state JSON: {public_state}\nAvailable intentions JSON: {actions}\nRelevant NPC knowledge and canon: {relevant} {route}")
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
        assert!(!context.contains("Savely"));
        assert!(context.contains("not yet been found"));
        assert!(context.contains("separate records"));
        assert!(!context.contains("sell its silver fitting"));
    }
    #[test]
    fn public_narration_facts_include_only_discoveries_and_share_the_ward_canon() {
        let mut state = AdventureState::new();
        state.location = Location::Warehouse;
        let unknown = serde_json::to_string(&bell_public_narration_facts(&state, false)).unwrap();
        for secret in ["Varden", "Savely", "tool marks", "silver fitting"] {
            assert!(!unknown.contains(secret));
        }
        state.location = Location::Quay;
        state.facts.insert(Fact::WardDamaged);
        let facts = bell_public_narration_facts(&state, true);
        let ward = facts
            .iter()
            .find(|fact| fact.id == "ward_inspection")
            .unwrap();
        assert!(ward.required);
        assert!(bell_dm_context(&state).contains(&ward.text));
        assert!(
            !bell_public_narration_facts(&state, false)
                .iter()
                .find(|fact| fact.id == "ward_inspection")
                .unwrap()
                .required
        );
        state.facts.insert(Fact::TikhonRescued);
        assert!(bell_public_narration_facts(&state, false)
            .iter()
            .any(|fact| fact.id == "tikhon_rescued" && fact.text.contains("at home")));
        state.package_id = "different-adventure".into();
        assert!(bell_public_narration_facts(&state, true).is_empty());
    }
}

#[cfg(test)]
mod narration_tests {
    use super::*;
    #[test]
    fn narration_brief_tracks_rescue_reward_and_discovery_without_changing_state() {
        let mut state = AdventureState::new();
        state.location = Location::Archive;
        let before = state.clone();
        assert!(!bell_dm_context(&state).contains("Savely"));
        assert_eq!(state, before);
        state.facts.insert(Fact::TikhonLocated);
        assert!(bell_dm_context(&state).contains("found alive"));
        state.facts.insert(Fact::TikhonRescued);
        state.facts.insert(Fact::RewardGranted);
        let context = bell_dm_context(&state);
        assert!(context.contains("at home with Mila"));
        assert!(context.contains("already granted once"));
        assert!(!context.contains("found alive in the upper chamber"));
        state.facts.insert(Fact::RosterAvailable);
        assert!(bell_dm_context(&state).contains("Savely / Савелий"));
    }
}

#[cfg(test)]
mod ending_tests {
    use super::*;
    #[test]
    fn closed_crossing_brief_distinguishes_physical_gate_from_magical_repair() {
        let mut state = AdventureState::new();
        state.closed = true;
        state.ending = Some(Ending::ClosedCrossing);
        state.facts.insert(Fact::LowerSluiceSealed);
        let context = bell_dm_context(&state);
        assert!(context.contains("trading ferry route is CLOSED"));
        assert!(context.contains("ward is still damaged"));
        assert!(context.contains("do not call it again"));
        let state = AdventureState::new();
        assert!(!bell_dm_context(&state).contains("The ward remains damaged"));
    }
}
