import { useTranslation } from 'react-i18next';
import type { AssistField } from '../../api/characterAssist';
import type { Compendium } from '../../api/srd';
import { useCharacterAssist } from '../../hooks/useCharacterAssist';
import { useStore } from '../../state/useStore';

const ALIGNMENTS: readonly (readonly string[])[] = [
  ['LG', 'NG', 'CG'],
  ['LN', 'TN', 'CN'],
  ['LE', 'NE', 'CE'],
];

export function PersonaTab(_props: { compendium: Compendium }) {
  const { t } = useTranslation('wizard');
  const name = useStore((s) => s.charCreation.name);
  const ideals = useStore((s) => s.charCreation.ideals);
  const bonds = useStore((s) => s.charCreation.bonds);
  const flaws = useStore((s) => s.charCreation.flaws);
  const backstory = useStore((s) => s.charCreation.backstory);
  const alignment = useStore((s) => s.charCreation.alignment);
  const isAssisting = useStore((s) => s.charCreation.isAssisting);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const { generateField } = useCharacterAssist();

  function sparkle(field: AssistField) {
    return (
      <button
        type="button"
        className="dm-wizard-sparkle"
        disabled={isAssisting}
        onClick={() => {
          void generateField(field);
        }}
      >
        {t('generate')}
      </button>
    );
  }

  return (
    <section>
      <h2>{t('persona_title')}</h2>

      <div style={{ marginBottom: 16 }}>
        <label htmlFor="dm-persona-name" className="dm-wizard-live-section-label">
          {t('name')}
        </label>
        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <input
            id="dm-persona-name"
            value={name}
            onChange={(e) => setDraftField('name', e.target.value)}
            style={{ flex: 1, padding: 8 }}
          />
          {sparkle('name')}
        </div>
      </div>

      <div style={{ marginBottom: 16 }}>
        <div className="dm-wizard-live-section-label">{t('alignment')}</div>
        <table style={{ borderCollapse: 'collapse', marginTop: 4 }}>
          <tbody>
            {ALIGNMENTS.map((row) => (
              <tr key={row.join('-')}>
                {row.map((a) => (
                  <td key={a} style={{ padding: 2 }}>
                    <button
                      type="button"
                      className={`dm-wizard-card${alignment === a ? ' is-selected' : ''}`}
                      style={{ minWidth: 56, textAlign: 'center' }}
                      onClick={() => setDraftField('alignment', a)}
                    >
                      {a}
                    </button>
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {(
        [
          ['ideals', ideals],
          ['bonds', bonds],
          ['flaws', flaws],
          ['backstory', backstory],
        ] as const
      ).map(([field, value]) => (
        <div key={field} style={{ marginBottom: 16 }}>
          <label htmlFor={`dm-persona-${field}`} className="dm-wizard-live-section-label">
            {t(field)}
          </label>
          <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
            <textarea
              id={`dm-persona-${field}`}
              value={value}
              onChange={(e) => setDraftField(field, e.target.value)}
              style={{ flex: 1, minHeight: 60, padding: 8 }}
            />
            {sparkle(field as AssistField)}
          </div>
        </div>
      ))}
    </section>
  );
}
