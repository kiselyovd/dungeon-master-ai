use std::collections::BTreeSet;

use app_domain::compendium::{character_catalog, CURRENT_RULESET_ID};

fn ids<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    values.into_iter().collect()
}

#[test]
fn catalog_has_the_official_srd_521_character_identity() {
    let catalog = character_catalog();

    assert_eq!(catalog.provenance.ruleset_id, CURRENT_RULESET_ID);
    assert_eq!(catalog.provenance.license_id, "CC-BY-4.0");
    assert_eq!(
        catalog.provenance.source_url,
        "https://www.dndbeyond.com/srd"
    );

    assert_eq!(
        ids(catalog.races.iter().map(|entry| entry.id.as_str())),
        ids([
            "dragonborn",
            "dwarf",
            "elf",
            "gnome",
            "goliath",
            "halfling",
            "human",
            "orc",
            "tiefling",
        ])
    );
    assert_eq!(
        ids(catalog.classes.iter().map(|entry| entry.id.as_str())),
        ids([
            "barbarian",
            "bard",
            "cleric",
            "druid",
            "fighter",
            "monk",
            "paladin",
            "ranger",
            "rogue",
            "sorcerer",
            "warlock",
            "wizard",
        ])
    );
    assert_eq!(
        ids(catalog.backgrounds.iter().map(|entry| entry.id.as_str())),
        ids(["acolyte", "criminal", "sage", "soldier"])
    );

    let legacy_species = ["half-elf", "half-orc"];
    assert!(legacy_species
        .iter()
        .all(|legacy| !catalog.races.iter().any(|entry| entry.id == *legacy)));
}

#[test]
fn every_supported_character_option_is_unique_and_attributed() {
    type AttributedOption<'a> = (&'a str, &'a str, &'a str);
    type CatalogCollection<'a> = (&'static str, Vec<AttributedOption<'a>>);

    let catalog = character_catalog();

    let collections: [CatalogCollection<'_>; 7] = [
        (
            "species",
            catalog
                .races
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
        (
            "backgrounds",
            catalog
                .backgrounds
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
        (
            "classes",
            catalog
                .classes
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
        (
            "feats",
            catalog
                .feats
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
        (
            "spells",
            catalog
                .spells
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
        (
            "weapon properties",
            catalog
                .weapon_properties
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
        (
            "weapon masteries",
            catalog
                .weapon_masteries
                .iter()
                .map(|entry| {
                    (
                        entry.id.as_str(),
                        entry.ruleset_id.as_str(),
                        entry.name_key.as_str(),
                    )
                })
                .collect(),
        ),
    ];

    for (kind, entries) in collections {
        let unique = ids(entries.iter().map(|(id, _, _)| *id));
        assert_eq!(unique.len(), entries.len(), "duplicate {kind} identifier");
        for (id, ruleset_id, name_key) in entries {
            assert_eq!(
                ruleset_id, CURRENT_RULESET_ID,
                "wrong ruleset for {kind} {id}"
            );
            assert!(
                name_key.starts_with("catalog."),
                "missing locale key for {kind} {id}"
            );
        }
    }
}

#[test]
fn background_feats_and_weapon_masteries_match_the_supported_srd_manifest() {
    let catalog = character_catalog();

    assert_eq!(
        ids(catalog.feats.iter().map(|entry| entry.id.as_str())),
        ids(["alert", "magic-initiate", "savage-attacker", "skilled"])
    );
    assert_eq!(
        ids(catalog
            .weapon_masteries
            .iter()
            .map(|entry| entry.id.as_str())),
        ids(["cleave", "graze", "nick", "push", "sap", "slow", "topple", "vex"])
    );
}

#[test]
fn server_executable_spells_keep_their_required_mechanics() {
    let catalog = character_catalog();

    for id in ["acid-splash", "burning-hands", "guiding-bolt", "shatter"] {
        let spell = catalog
            .spells
            .iter()
            .find(|spell| spell.id == id)
            .unwrap_or_else(|| panic!("missing executable spell {id}"));
        assert!(spell.damage.is_some(), "missing damage mechanics for {id}");
    }

    let cure_wounds = catalog
        .spells
        .iter()
        .find(|spell| spell.id == "cure-wounds")
        .expect("missing executable healing spell");
    assert!(
        cure_wounds.description_en.contains("2d8"),
        "healing spell must retain parseable SRD 5.2.1 dice"
    );
}
