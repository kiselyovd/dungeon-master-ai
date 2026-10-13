import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { isDeadToken } from '../features/combat/model/selectors';
import type { CombatToken } from '../state/combat';
import { Icons } from '../ui/Icons';

interface Props {
  selectedTokenId?: string | null | undefined;
  compact?: boolean;
  tokens: CombatToken[];
  order: string[];
  round: number;
  activeTokenId?: string | null;
  onSelect?: (tokenId: string) => void;
}

/**
 * InitiativeTracker overlay placed top-left over the VTT during combat.
 * Caller is responsible for not rendering this when combat is inactive.
 */
export function InitiativeTracker({
  selectedTokenId,
  tokens,
  order,
  round,
  activeTokenId,
  onSelect,
  compact = false,
}: Props) {
  const { t } = useTranslation('combat');
  const [expanded, setExpanded] = useState(false);

  const ordered = order
    .map((id) => tokens.find((tok) => tok.id === id))
    .filter((tok): tok is CombatToken => tok !== undefined);

  if (ordered.length === 0) return null;

  const activeIndex = ordered.findIndex((tok) => tok.id === activeTokenId);

  return (
    <aside
      className={`dm-init-tracker${compact ? ' dm-init-compact' : ''}`}
      aria-label={t('initiative_tracker')}
    >
      <div className="dm-init-header">
        <div className="dm-init-title">
          <Icons.D20 size={14} />
          <span>{t('initiative_tracker')}</span>
        </div>
        <div className="dm-init-round">{t('round', { round })}</div>
      </div>
      {compact && (
        <button
          className="dm-init-summary"
          type="button"
          aria-expanded={expanded}
          onClick={() => setExpanded((value) => !value)}
        >
          <span>
            {ordered[activeIndex]?.displayNameKey
              ? t(ordered[activeIndex]?.displayNameKey ?? '', {
                  defaultValue: ordered[activeIndex]?.name,
                })
              : (ordered[activeIndex]?.name ?? t('initiative_tracker'))}
          </span>
          <Icons.ChevronDown size={14} />
        </button>
      )}
      {(!compact || expanded) && (
        <ol className="dm-init-list">
          {ordered.map((tok, index) => {
            const isActive =
              activeTokenId === tok.id || (activeTokenId === undefined && index === 0);
            const isDone = activeIndex >= 0 && index < activeIndex;
            const isDead = isDeadToken(tok);
            const hpPct = tok.maxHp > 0 ? (tok.hp / tok.maxHp) * 100 : 0;
            const hpClass = hpPct < 25 ? 'crit' : hpPct < 50 ? 'low' : '';
            return (
              <li key={tok.id} className="dm-init-item">
                <button
                  type="button"
                  data-active={isActive ? 'true' : undefined}
                  className={`dm-init-card${isActive ? ' is-active' : ''}${isDone ? ' is-done' : ''}`}
                  onClick={() => onSelect?.(tok.id)}
                  aria-pressed={onSelect ? selectedTokenId === tok.id : undefined}
                  aria-label={t('token_aria', {
                    name: tok.displayNameKey
                      ? t(tok.displayNameKey, { defaultValue: tok.name })
                      : tok.name,
                    hp: tok.hp,
                    maxHp: tok.maxHp,
                    ac: tok.ac,
                  })}
                >
                  <div className="dm-init-init">{index + 1}</div>
                  <div className="dm-init-body">
                    <div className={`dm-init-name${isDead ? ' is-dead' : ''}`}>
                      {tok.displayNameKey
                        ? t(tok.displayNameKey, { defaultValue: tok.name })
                        : tok.name}
                    </div>
                    <div className="dm-init-meta">
                      <span className="dm-init-ac" title={t('ac_label')}>
                        <Icons.Shield size={10} />
                        <span>{tok.ac}</span>
                      </span>
                      <span>
                        {tok.hp}/{tok.maxHp}
                      </span>
                    </div>
                    <div className="dm-hpbar">
                      <div
                        className={`dm-hpbar-fill${hpClass !== '' ? ` ${hpClass}` : ''}`}
                        style={{ width: `${hpPct}%` }}
                      />
                    </div>
                  </div>
                </button>
              </li>
            );
          })}
        </ol>
      )}
    </aside>
  );
}
