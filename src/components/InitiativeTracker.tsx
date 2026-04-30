import { useTranslation } from 'react-i18next';
import type { CombatToken } from '../state/combat';

interface Props {
  tokens: CombatToken[];
  order: string[];
  round: number;
  visible: boolean;
}

/**
 * InitiativeTracker slides in from the right when combat is active.
 * The slide-in transition is `transform: translateX(0)` over `var(--t-slow)`
 * (280ms), wired in `src/styles/combat.css`.
 */
export function InitiativeTracker({ tokens, order, round, visible }: Props) {
  const { t } = useTranslation('combat');

  const ordered = order
    .map((id) => tokens.find((tok) => tok.id === id))
    .filter((tok): tok is CombatToken => tok !== undefined);

  return (
    <aside className={`initiative-tracker${visible ? ' visible' : ''}`}>
      <h3
        style={{
          margin: '0 0 var(--space-2) 0',
          fontSize: 'var(--text-sm)',
          fontFamily: 'var(--font-display)',
          letterSpacing: 'var(--tracking-display)',
          color: 'var(--color-accent)',
        }}
      >
        {t('round', { round })}
      </h3>

      <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
        {ordered.map((tok) => (
          <li
            key={tok.id}
            data-active={tok.isActive ? 'true' : undefined}
            className={`initiative-entry${tok.isActive ? ' active' : ''}`}
          >
            <span style={{ flex: 1, fontSize: 'var(--text-sm)' }}>{tok.name}</span>
            <span
              style={{
                fontSize: 'var(--text-xs)',
                fontFamily: 'var(--font-mono)',
                color: tok.hp === 0 ? 'var(--color-danger)' : 'var(--color-fg-secondary)',
              }}
            >
              {tok.hp}/{tok.maxHp}
            </span>
          </li>
        ))}
      </ul>
    </aside>
  );
}
