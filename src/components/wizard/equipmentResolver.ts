/**
 * Equipment resolver utility - turns CharacterDraft equipment data into
 * concrete InventoryItem rows for pc.inventory at Begin Adventure.
 *
 * Pure module: no React, no Zustand, no I/O. Exports parse / lookup helpers
 * as well as the top-level resolveEquipmentSlots aggregator. See
 * docs/superpowers/specs/2026-05-13-equipment-resolver-design.md for the
 * decisions behind the heuristics.
 */

export interface ParsedItemDescriptor {
  count: number;
  nameKey: string;
  isWildcard: boolean;
  ifProficient: boolean;
}

const NUMBER_WORDS: Record<string, number> = {
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
  nine: 9,
  ten: 10,
};

const SAFE_PLURAL_STRIP: ReadonlySet<string> = new Set([
  'handaxes',
  'javelins',
  'daggers',
  'darts',
]);

/**
 * Parse a raw SRD starting-equipment string into a structured descriptor.
 *
 * Handles:
 * - leading articles ("a", "an", "the")
 * - trailing parenthetical descriptions ("(a gift to you...)")
 * - count expressed as either a number-word ("two") or digits ("20")
 * - the "if proficient" qualifier (set as a flag, stripped from nameKey)
 * - the "any X" wildcard prefix
 * - ammo bundles: "N bolts" / "N arrows" remap to the catalog's bundled
 *   `crossbow-bolts-20` / `arrows-20` ids with count=1
 * - naive plural strip (handaxes -> handaxe, javelins -> javelin)
 */
export function parseEquipmentString(raw: string): ParsedItemDescriptor {
  let working = raw.trim().toLowerCase();

  // 1. ifProficient suffix
  let ifProficient = false;
  if (/\(if proficient\)\s*$/.test(working)) {
    ifProficient = true;
    working = working.replace(/\s*\(if proficient\)\s*$/, '').trim();
  }

  // 2. strip leading articles
  working = working.replace(/^(a|an|the)\s+/, '');

  // 3. strip trailing parenthetical
  working = working.replace(/\s*\([^)]*\)\s*$/, '').trim();

  // 4. count parse - try number word first, then digits
  let count = 1;
  for (const [word, value] of Object.entries(NUMBER_WORDS)) {
    const re = new RegExp(`^${word}\\s+`);
    if (re.test(working)) {
      count = value;
      working = working.replace(re, '');
      break;
    }
  }
  if (count === 1) {
    const m = working.match(/^(\d+)\s+/);
    if (m && m[1]) {
      count = parseInt(m[1], 10);
      working = working.replace(/^\d+\s+/, '');
    }
  }

  // 5. wildcard
  let isWildcard = false;
  if (working.startsWith('any ')) {
    isWildcard = true;
    working = working.slice('any '.length);
  }

  // 6. ammo bundle alias
  if (count > 1) {
    if (working === 'bolts' || working === 'bolt') {
      return {
        count: 1,
        nameKey: 'crossbow-bolts-20',
        isWildcard,
        ifProficient,
      };
    }
    if (working === 'arrows' || working === 'arrow') {
      return {
        count: 1,
        nameKey: 'arrows-20',
        isWildcard,
        ifProficient,
      };
    }
  }

  // 7. naive plural strip (skip for wildcard nameKeys)
  if (!isWildcard && SAFE_PLURAL_STRIP.has(working)) {
    working = working.slice(0, -1);
  }

  return {
    count,
    nameKey: working,
    isWildcard,
    ifProficient,
  };
}
