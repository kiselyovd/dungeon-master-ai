import { useTranslation } from 'react-i18next';
import type { CombatToken } from '../state/combat';
import { Icons } from '../ui/Icons';

interface Props {
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
export function InitiativeTracker({ tokens, order, round, activeTokenId, onSelect }: Props) {
  const { t } = useTranslation('combat');

  const ordered = order
    .map((id) => tokens.find((tok) => tok.id === id))
    .filter((tok): tok is CombatToken => tok !== undefined);

  if (ordered.length === 0) return null;

  const activeIndex = ordered.findIndex((tok) => tok.id === activeTokenId);

  return (
    <aside className="dm-init-tracker" aria-label={t('initiative_tracker')}>
      <div className="dm-init-header">
        <div className="dm-init-title">
          <Icons.D20 size={14} />
          <span>{t('initiative_tracker')}</span>
        </div>
        <div className="dm-init-round">{t('round', { round })}</div>
      </div>
      <ol className="dm-init-list">
        {ordered.map((tok, index) => {
          const isActive = activeTokenId === tok.id || (activeTokenId === undefined && index === 0);
          const isDone = activeIndex >= 0 && index < activeIndex;
          const hpPct = tok.maxHp > 0 ? (tok.hp / tok.maxHp) * 100 : 0;
          const hpClass = hpPct < 25 ? 'crit' : hpPct < 50 ? 'low' : '';
          return (
            <li key={tok.id} className="dm-init-item">
              <button
                type="button"
                data-active={isActive ? 'true' : undefined}
                className={`dm-init-card${isActive ? ' is-active' : ''}${isDone ? ' is-done' : ''}`}
                onClick={() => onSelect?.(tok.id)}
                aria-label={`${tok.name} ${tok.hp}/${tok.maxHp} HP, AC ${tok.ac}`}
              >
                <div className="dm-init-init">{index + 1}</div>
                <div className="dm-init-body">
                  <div className="dm-init-name">{tok.name}</div>
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
    </aside>
  );
}
