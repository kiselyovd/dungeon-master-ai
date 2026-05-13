import { useTranslation } from 'react-i18next';
import type { AbilityMethod } from '../../state/charCreation';
import type { AbilityScores } from '../../state/pc';
import { useStore } from '../../state/useStore';

const STANDARD_ARRAY = [15, 14, 13, 12, 10, 8] as const;
const POINT_BUY_COSTS: Record<number, number> = {
  8: 0,
  9: 1,
  10: 2,
  11: 3,
  12: 4,
  13: 5,
  14: 7,
  15: 9,
};
const ABILITIES: readonly (keyof AbilityScores)[] = ['str', 'dex', 'con', 'int', 'wis', 'cha'];
const POINT_BUY_BUDGET = 27;
const MAX_ROLLS = 3;

function pointBuyRemaining(scores: AbilityScores): number {
  return (
    POINT_BUY_BUDGET - ABILITIES.reduce((acc, k) => acc + (POINT_BUY_COSTS[scores[k]] ?? 0), 0)
  );
}

export function AbilitiesTab() {
  const { t } = useTranslation('wizard');
  const method = useStore((s) => s.charCreation.abilityMethod);
  const abilities = useStore((s) => s.charCreation.abilities);
  const history = useStore((s) => s.charCreation.abilityRollHistory);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const setAbilityScore = useStore((s) => s.charCreation.setAbilityScore);
  const rollAbilityScores = useStore((s) => s.charCreation.rollAbilityScores);

  const remaining = pointBuyRemaining(abilities);

  function pickMethod(m: AbilityMethod) {
    setDraftField('abilityMethod', m);
    if (m === 'point_buy' || m === 'standard_array') {
      // Reset to a neutral baseline so users can re-assign cleanly.
      setDraftField('abilities', { str: 10, dex: 10, con: 10, int: 10, wis: 10, cha: 10 });
    }
  }

  return (
    <section>
      <h2>{t('abilities_title')}</h2>
      <div
        role="radiogroup"
        aria-label={t('abilities_method_label')}
        style={{ display: 'flex', gap: 8, marginBottom: 16 }}
      >
        {(['point_buy', 'standard_array', '4d6_drop_lowest'] as AbilityMethod[]).map((m) => (
          // biome-ignore lint/a11y/useSemanticElements: card-styled radio
          <button
            key={m}
            type="button"
            role="radio"
            aria-checked={method === m}
            className={`dm-wizard-card${method === m ? ' is-selected' : ''}`}
            onClick={() => pickMethod(m)}
          >
            {t(`ability_method_${m}`)}
          </button>
        ))}
      </div>

      {method === 'point_buy' && (
        <PointBuyPanel abilities={abilities} remaining={remaining} onChange={setAbilityScore} />
      )}

      {method === 'standard_array' && (
        <StandardArrayPanel abilities={abilities} onChange={setAbilityScore} />
      )}

      {method === '4d6_drop_lowest' && (
        <RollPanel
          abilities={abilities}
          history={history}
          onRoll={rollAbilityScores}
          onChange={setAbilityScore}
        />
      )}
    </section>
  );
}

interface PanelProps {
  abilities: AbilityScores;
  onChange: (k: keyof AbilityScores, v: number) => void;
}

function PointBuyPanel({ abilities, remaining, onChange }: PanelProps & { remaining: number }) {
  const { t } = useTranslation('wizard');
  return (
    <div>
      <div style={{ marginBottom: 8, color: 'var(--color-accent)' }}>
        {t('point_buy_remaining', { remaining })}
      </div>
      {ABILITIES.map((k) => (
        <div
          key={k}
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            padding: 4,
          }}
        >
          <span>{t(`ability_${k}`)}</span>
          <span style={{ display: 'inline-flex', gap: 8, alignItems: 'center' }}>
            <button
              type="button"
              aria-label={t('decrement', { ability: k })}
              disabled={abilities[k] <= 8}
              onClick={() => onChange(k, Math.max(8, abilities[k] - 1))}
            >
              -
            </button>
            <span style={{ minWidth: 24, textAlign: 'center' }}>{abilities[k]}</span>
            <button
              type="button"
              aria-label={t('increment', { ability: k })}
              disabled={
                abilities[k] >= 15 ||
                remaining -
                  ((POINT_BUY_COSTS[abilities[k] + 1] ?? 99) -
                    (POINT_BUY_COSTS[abilities[k]] ?? 0)) <
                  0
              }
              onClick={() => onChange(k, Math.min(15, abilities[k] + 1))}
            >
              +
            </button>
          </span>
        </div>
      ))}
    </div>
  );
}

function StandardArrayPanel({ abilities, onChange }: PanelProps) {
  const { t } = useTranslation('wizard');
  return (
    <div>
      <p style={{ fontSize: 12 }}>{t('standard_array_hint')}</p>
      <div style={{ display: 'flex', gap: 8, marginBottom: 12, flexWrap: 'wrap' }}>
        {STANDARD_ARRAY.map((v, idx) => (
          <span key={idx} className="dm-wizard-card" style={{ minWidth: 48, textAlign: 'center' }}>
            {v}
          </span>
        ))}
      </div>
      {ABILITIES.map((k) => (
        <div key={k} style={{ display: 'flex', justifyContent: 'space-between', padding: 4 }}>
          <label htmlFor={`ab-${k}`}>{t(`ability_${k}`)}</label>
          <select
            id={`ab-${k}`}
            value={abilities[k]}
            onChange={(e) => onChange(k, Number(e.target.value))}
          >
            {STANDARD_ARRAY.map((v) => (
              <option key={v} value={v}>
                {v}
              </option>
            ))}
          </select>
        </div>
      ))}
    </div>
  );
}

function RollPanel({
  abilities,
  history,
  onRoll,
  onChange,
}: PanelProps & { history: number[][]; onRoll: () => void }) {
  const { t } = useTranslation('wizard');
  const remaining = MAX_ROLLS - history.length;
  return (
    <div>
      <button
        type="button"
        className="dm-wizard-btn-secondary"
        disabled={remaining <= 0}
        onClick={() => onRoll()}
      >
        {t('roll_set', { remaining })}
      </button>
      {history.map((set, idx) => (
        <div key={idx} style={{ display: 'flex', gap: 8, marginTop: 8 }}>
          {set.map((v, i) => (
            <span key={i} className="dm-wizard-card" style={{ minWidth: 36, textAlign: 'center' }}>
              {v}
            </span>
          ))}
        </div>
      ))}
      <div style={{ marginTop: 16 }}>
        {ABILITIES.map((k) => (
          <div key={k} style={{ display: 'flex', justifyContent: 'space-between', padding: 4 }}>
            <label htmlFor={`ab-roll-${k}`}>{t(`ability_${k}`)}</label>
            <input
              id={`ab-roll-${k}`}
              type="number"
              min={3}
              max={18}
              value={abilities[k]}
              onChange={(e) => onChange(k, Number(e.target.value))}
              style={{ width: 60 }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
