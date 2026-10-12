export interface PublicToolOutcome {
  key: 'outcome_dice' | 'outcome_hit_points' | 'outcome_done';
  values: Record<string, number>;
}

/** Whitelisted mechanical numbers only. Provider text and arbitrary JSON never
 * become public narration through this adapter. Unknown shapes stay generic. */
export function publicToolOutcome(toolName: string, result: unknown): PublicToolOutcome {
  const fallback: PublicToolOutcome = { key: 'outcome_done', values: {} };
  if (!result || typeof result !== 'object' || Array.isArray(result)) return fallback;
  const record = result as Record<string, unknown>;
  const integer = (value: unknown): value is number =>
    typeof value === 'number' && Number.isSafeInteger(value);
  if (toolName === 'roll_dice' && integer(record.total))
    return { key: 'outcome_dice', values: { total: record.total } };
  if (
    ['apply_damage', 'apply_healing'].includes(toolName) &&
    integer(record.new_hp) &&
    record.new_hp >= 0
  )
    return { key: 'outcome_hit_points', values: { hp: record.new_hp } };
  return fallback;
}
