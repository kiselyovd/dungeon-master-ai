import type { CombatToken, SpellChoiceDto } from './types';
export function spellAvailable(actor: CombatToken, spell: SpellChoiceDto, slot: number): boolean {
  return (
    spell.supported &&
    (spell.bonus_action ? actor.bonusAvailable !== false : actor.actionAvailable !== false) &&
    (spell.level === 0
      ? slot === 0
      : !actor.spellSlotUsed &&
        slot >= spell.level &&
        (actor.casting?.slots[String(slot)]?.remaining ?? 0) > 0)
  );
}
export function canCast(actor: CombatToken): boolean {
  return (
    actor.casting?.choices.some((spell) =>
      spell.level === 0
        ? spellAvailable(actor, spell, 0)
        : Object.keys(actor.casting?.slots ?? {}).some((level) =>
            spellAvailable(actor, spell, Number(level)),
          ),
    ) ?? false
  );
}
