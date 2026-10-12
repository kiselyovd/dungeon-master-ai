import { emptyPcData, type HeroClassId, presetToData } from '../../../state/pc';

// Existing ability/HP presets with an explicit SRD loadout. The server derives
// defense and owns all resource and combat changes after confirmation.
export function preparedHero(classId: HeroClassId) {
  const base = presetToData(classId);
  const armor = {
    fighter: 'chain-mail',
    rogue: 'leather-armor',
    cleric: 'scale-mail',
    wizard: null,
  }[classId];
  const spells =
    classId === 'wizard'
      ? {
          cantrips: ['fire-bolt', 'mage-hand', 'light'],
          level1: ['magic-missile', 'shield', 'sleep', 'detect-magic'],
        }
      : classId === 'cleric'
        ? {
            cantrips: ['sacred-flame', 'guidance', 'light'],
            level1: ['bless', 'cure-wounds', 'guiding-bolt', 'healing-word'],
          }
        : { cantrips: [], level1: [] };
  return {
    ...emptyPcData(),
    ...base,
    race: { fighter: 'human', rogue: 'halfling', wizard: 'elf', cleric: 'dwarf' }[classId],
    background: base.background?.toLowerCase() ?? null,
    inventory: [
      ...base.inventory.map((item) => ({ ...item, id: item.id.replace(`${classId}-`, '') })),
      ...(armor ? [{ id: armor, name: armor, count: 1, icon: 'shield' }] : []),
    ],
    defense: {
      schema_version: 1 as const,
      armor_id: armor,
      shield: classId === 'fighter' || classId === 'cleric',
    },
    ac: classId === 'fighter' ? 18 : base.ac,
    baseSpeedFt: base.speedFt,
    spells,
  };
}
