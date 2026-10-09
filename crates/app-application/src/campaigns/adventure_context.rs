//! Shared discovered narration facts and separate private DM context.
use app_domain::adventure::{AdventureState, Ending, Fact, Location};

const EXAMINED_WARD: &str = "During the confirmed inspection AT THE OLD QUAY (старый причал), fresh tool marks were found ON THE WARD (оберег), where the bell clapper (язык колокола) had been cut off and removed. The tool marks are on the ward itself, not on a wall or railings. The inspection did not locate the removed clapper. / При подтверждённом осмотре НА СТАРОМ ПРИЧАЛЕ было обнаружено: оберег был повреждён; на самом обереге свежие следы инструмента в месте, где срезали и сняли язык колокола. Следы не на стене и не на перилах. Где снятый язык находится, осмотр не установил. This is a discovery at the old quay, not a new inspection at another location. It does not establish an observation of the ward in a different current scene. Only damage to the ward is confirmed; no damage to other river infrastructure is established by this discovery. Flooding is a consequence, not proof of broken gates. No material, plate construction, extra damage, residue or exact tool is confirmed. / Это прежнее открытие на причале, не новый осмотр в другом месте. Подтверждено повреждение оберега, не других сооружений переправы. Затопление не доказывает поломку затворов. Материал, устройство пластин, дополнительные повреждения, налёт и точное орудие не установлены. This historical discovery does not override later confirmed recovery or restoration.";
const QUAY_UPPER_PATH: &str = "The maintained dry upper path connects the old quay and the archive safely; a failed check never blocks every rescue route. / Сухая верхняя тропа безопасно соединяет старый причал и архив; неудачная проверка не перекрывает все пути спасения.";

const RESCUE_REWARD: &str = "Exactly ten gold were already granted once to the rescuing PLAYER-CONTROLLED HERO, not to the rescued ferryman or his daughter. No further reward is confirmed. / Ровно десять золотых уже один раз выданы игровому герою, спасшему паромщика, а не самому паромщику или его дочери. Дополнительной награды нет.";

/// Public route aliases and their canonical parameters, never player permission.
pub fn bell_public_route_references(state: &AdventureState) -> Vec<serde_json::Value> {
    use app_domain::adventure::Action;
    if state.package_id != app_domain::adventure::BELL_ADVENTURE_ID {
        return vec![];
    }
    let destination = match state.location {
        Location::Quay => Location::Archive,
        Location::Archive => Location::Quay,
        _ => return vec![],
    };
    let action = Action::Move(destination);
    vec![serde_json::json!({
        "from":state.location,"to":destination,"description":QUAY_UPPER_PATH,
        "tool_name":"adventure_action","args":{"action":action},
        "currently_available":state.available_actions().contains(&action),
    })]
}

/// Meanings of currently offered canonical actions, without undiscovered outcomes.
pub fn bell_public_action_references(state: &AdventureState) -> Vec<serde_json::Value> {
    use app_domain::adventure::Action;
    state.available_actions().into_iter().filter_map(|action| {
        let purpose = match action {
            Action::Move(_) => return None,
            Action::InspectQuay => "Inspect the ward at the old quay. / Осмотреть оберег на старом причале.",
            Action::TalkAgata => "Talk to Agata about the quay and porters. / Поговорить с Агатой о причале и грузчиках.",
            Action::ConsultLeon => "Consult Leon about the ward and memorial records. / Расспросить Леона об обереге и памятных записях.",
            Action::NegotiateClapper => "Seek recovery of the missing clapper through negotiation. / Договориться о возвращении пропавшего языка колокола.",
            Action::RetrieveClapperPublicly => "Seek recovery of the missing clapper through public intervention. / Добиться возвращения языка колокола публичным вмешательством.",
            Action::RepairClapper => "Repair the recovered clapper. / Починить найденный язык колокола.",
            Action::LocateTikhon => "Call out or investigate the upper chamber looking for the missing ferryman Tikhon. This establishes no discovery until an engine result. / Окликнуть или разыскать пропавшего паромщика Тихона в верхней комнате; находка подтверждается только результатом действия.",
            Action::EscortTikhonHome => "Personally escort Tikhon home by the maintained dry upper route through the quay to Veresk. The HUMAN HERO also arrives in town. / Лично проводить Тихона домой по сухому верхнему пути через причал в Вереск. Игровой герой тоже прибывает в город.",
            Action::RescueTikhon => "Help the found ferryman return safely home by the upper route. This canonical action does not change the hero's location. / Помочь найденному паромщику безопасно добраться домой по верхнему пути. Это действие не меняет местоположение героя.",
            Action::RecoverRoster => "Recover the original memorial roll. / Получить оригинал памятного списка спасённых.",
            Action::RecoverLedger => "Recover the land-ownership ledger. / Получить земельную книгу.",
            Action::AcknowledgeEcho => "Address and acknowledge the echo. / Обратиться к отголоску и признать его.",
            Action::RestoreWard => "Restore the ward using the required recovered ritual objects and knowledge. / Восстановить оберег с помощью необходимых найденных предметов и сведений.",
            Action::SealSluice => "Close the existing lower sluice gates with their built-in mechanism. / Перекрыть нижний шлюз его существующим механизмом.",
            Action::PublishLedger => "Publish the recovered land ledger in town. / Обнародовать найденную земельную книгу в городе.",
            Action::Finish => "Record the actual outcome and end the session. / Записать фактический итог и завершить сессию.",
            Action::StartDefense => "Begin the optional defensive encounter. / Начать защитный бой.",
        };
        Some(serde_json::json!({"tool_name":"adventure_action","args":{"action":action},"purpose":purpose,"currently_available":true}))
    }).collect()
}

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
        add("rescue_reward", RESCUE_REWARD, false);
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
        RESCUE_REWARD
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
    } else if state.facts.contains(&Fact::LowerSluiceSealed) {
        "The ward remains damaged. The lower sluice is CONFIRMED SEALED, but that did not banish or silence the echo. / Оберег повреждён. Нижний шлюз подтверждённо закрыт, но это не изгнало и не заставило замолчать отголосок."
    } else if state.facts.contains(&Fact::WardDamaged)
        || state.facts.contains(&Fact::WardUnderstood)
    {
        "The ward is confirmed damaged. NO sluice closure is confirmed; the river danger and echo remain unresolved. Damage to the ward is not damage to the sluice. / Подтверждено повреждение оберега. Закрытие шлюза НЕ подтверждено; опасность реки и отголосок пока не разрешены. Повреждение оберега не означает поломку шлюза."
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
        state.facts.insert(Fact::RewardGranted);
        let rewarded = bell_public_narration_facts(&state, false);
        let reward = rewarded
            .iter()
            .find(|fact| fact.id == "rescue_reward")
            .unwrap();
        assert!(reward.text.contains("PLAYER-CONTROLLED HERO"));
        assert!(reward.text.contains("not to the rescued ferryman"));
        assert!(bell_dm_context(&state).contains(&reward.text));
        assert!(ward
            .text
            .contains("no damage to other river infrastructure"));
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
    #[test]
    fn damage_does_not_claim_sluice_closure_or_reveal_an_unexamined_cause() {
        let mut state = AdventureState::new();
        state.location = Location::Archive;
        state.facts.insert(Fact::WardDamaged);
        let open = bell_dm_context(&state);
        assert!(open.contains("NO sluice closure is confirmed"));
        assert!(!open.contains("CONFIRMED SEALED"));
        state.facts.insert(Fact::LowerSluiceSealed);
        assert!(bell_dm_context(&state).contains("CONFIRMED SEALED"));
        let mut debt = AdventureState::new();
        debt.closed = true;
        debt.ending = Some(Ending::UnfinishedDebt);
        let context = bell_dm_context(&debt);
        assert!(context.contains("ward has not been examined"));
        assert!(!context.contains("The ward remains damaged"));
    }
}
