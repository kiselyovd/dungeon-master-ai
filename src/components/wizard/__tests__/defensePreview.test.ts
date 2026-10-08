import { describe, expect, it } from 'vitest';
import catalog from '../../../../crates/app-domain/src/compendium/data/srd_5_2_1.yaml?raw';
import type { Compendium } from '../../../api/srd';
import { EMPTY_DRAFT } from '../../../state/charCreation';
import { computeLiveSheet } from '../computeLiveSheet';
import { deriveDefensePreview } from '../defensePreview';

const compendium = JSON.parse(catalog) as Compendium;
const abilities = { str: 12, dex: 14, con: 16, int: 10, wis: 16, cha: 10 };
const gear = ['chain-mail', 'shield', 'hide-armor'].map((id) => ({ id, name: id, count: 1 }));
const derive = (
  classId: string,
  scores = abilities,
  choice = { schema_version: 1 as const, armor_id: null as string | null, shield: false },
) =>
  deriveDefensePreview(
    classId,
    compendium.classes.find((c) => c.id === classId)?.armor_proficiencies ?? [],
    scores,
    gear,
    compendium.equipment.armor,
    choice,
  );

describe('defense preview uses the server catalog and explicit worn choice', () => {
  it('matches heavy armor, shield, Strength penalty and carried-only rules', () => {
    expect(
      derive('fighter', abilities, { schema_version: 1, armor_id: 'chain-mail', shield: true }),
    ).toMatchObject({ ac: 18, speedPenaltyFt: 10, valid: true, stealthDisadvantage: true });
    expect(derive('fighter')).toMatchObject({ ac: 12, speedPenaltyFt: 0 });
    expect(
      derive(
        'fighter',
        { ...abilities, dex: 8 },
        { schema_version: 1, armor_id: 'chain-mail', shield: false },
      ).ac,
    ).toBe(16);
    expect(
      derive(
        'fighter',
        { ...abilities, dex: 8 },
        { schema_version: 1, armor_id: 'hide-armor', shield: false },
      ).ac,
    ).toBe(11);
  });
  it('accepts Rust wire null for unlimited Dexterity and absent Strength requirement', () => {
    const armor = compendium.equipment.armor.map((item) => ({
      ...item,
      ac_dex_bonus_cap: item.ac_dex_bonus_cap ?? null,
      str_req: item.str_req ?? null,
    }));
    expect(
      deriveDefensePreview(
        'rogue',
        ['light'],
        abilities,
        [{ id: 'leather-armor', name: 'Leather Armor', count: 1 }],
        armor,
        { schema_version: 1, armor_id: 'leather-armor', shield: false },
      ),
    ).toMatchObject({ ac: 13, valid: true, speedPenaltyFt: 0 });
  });
  it('matches class defenses and rejects unavailable player choices', () => {
    expect(derive('barbarian').ac).toBe(15);
    expect(derive('monk').ac).toBe(15);
    expect(
      derive('wizard', abilities, { schema_version: 1, armor_id: 'chain-mail', shield: false })
        .valid,
    ).toBe(false);
    expect(
      derive('fighter', abilities, { schema_version: 1, armor_id: 'plate-armor', shield: false })
        .valid,
    ).toBe(false);
  });
  it('resolves package options before calculating live AC and movement', () => {
    const sheet = computeLiveSheet(
      {
        ...EMPTY_DRAFT,
        classId: 'fighter',
        raceId: 'human',
        abilities,
        equipmentMode: 'package',
        equipmentSlots: [
          {
            slotId: 'class-0',
            category: 'gear',
            itemId: 'A',
            customName: 'Chain Mail, Shield',
            fromBackground: false,
            resolvedItemIds: [],
          },
        ],
      },
      compendium,
    );
    expect(sheet).toMatchObject({
      ac: 18,
      baseSpeedFt: 30,
      speedFt: 20,
      defense: { choice: { armor_id: 'chain-mail', shield: true }, valid: true },
    });
  });
});
