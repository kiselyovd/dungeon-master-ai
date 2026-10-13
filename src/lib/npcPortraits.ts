import type { NpcRecord } from '../state/npc';

export const NPC_ARCHETYPES = [
  'innkeeper',
  'guard',
  'merchant',
  'rogue',
  'mage',
  'priestess',
  'knight',
  'peasant',
] as const;
export type NpcArchetype = (typeof NPC_ARCHETYPES)[number];

const ROLE_KEYWORDS: ReadonlyArray<readonly [RegExp, NpcArchetype]> = [
  [/innkeep|tavern|barkeep|bartender|host|трактир|корчм|хозяин.*таверн/i, 'innkeeper'],
  [/priest|cleric|nun|abbot|bishop|temple|shrine|жрец|жриц|священ|монах|храм/i, 'priestess'],
  [/wizard|mage|sorcer|warlock|witch|arcanist|scholar|волшеб|маг|колдун|чарод|учён/i, 'mage'],
  [/knight|paladin|champion|captain|baron|lord|рыцар|паладин|капитан|барон/i, 'knight'],
  [/guard|soldier|sentinel|watchman|sergeant|warden|страж|солдат|дозор|караул|сержант/i, 'guard'],
  [/rogue|thief|bandit|brigand|cutpurse|assassin|spy|плут|вор|разбой|бандит|шпион/i, 'rogue'],
  [
    /merchant|trader|shopkeep|vendor|smith|fletcher|alchem|crafts|купец|торгов|лавоч|кузнец|ремесл|алхим/i,
    'merchant',
  ],
  [
    /farm|peasant|villager|commoner|laborer|miner|sailor|кресть|фермер|земледел|рабоч|шахт|моряк|селянин/i,
    'peasant',
  ],
];
function stableHash(id: string): number {
  let hash = 0x811c9dc5;
  for (let index = 0; index < id.length; index++) {
    hash ^= id.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}
/** Classification suggests an initial appearance; persisted ID remains authoritative. */
export function npcArchetype(npc: Pick<NpcRecord, 'id' | 'role'>): NpcArchetype {
  for (const [pattern, archetype] of ROLE_KEYWORDS) if (pattern.test(npc.role)) return archetype;
  // Keep the original eight-way fallback invariant as the catalog grows.
  return NPC_ARCHETYPES[stableHash(npc.id) % 8] ?? 'peasant';
}
export function genericNpcPortraitId(npc: Pick<NpcRecord, 'id' | 'role'>): string {
  return `npc.${npcArchetype(npc)}.v2`;
}
