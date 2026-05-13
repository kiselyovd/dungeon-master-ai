import { useTranslation } from 'react-i18next';
import type { Class, Compendium } from '../../api/srd';
import { useStore } from '../../state/useStore';

interface ClassSkillData {
  choose?: number;
  from?: string[];
}

function readSkillProfData(klass: Class): ClassSkillData {
  const raw = (klass as unknown as { skill_proficiencies?: unknown }).skill_proficiencies;
  if (typeof raw !== 'object' || raw === null) return {};
  const obj = raw as { choose?: unknown; from?: unknown };
  const choose = typeof obj.choose === 'number' ? obj.choose : undefined;
  const from =
    Array.isArray(obj.from) && obj.from.every((x) => typeof x === 'string')
      ? (obj.from as string[])
      : undefined;
  const result: ClassSkillData = {};
  if (choose !== undefined) result.choose = choose;
  if (from !== undefined) result.from = from;
  return result;
}

export function SkillsTab({ compendium }: { compendium: Compendium }) {
  const { t } = useTranslation('wizard');
  const classId = useStore((s) => s.charCreation.classId);
  const backgroundId = useStore((s) => s.charCreation.backgroundId);
  const skillProfs = useStore((s) => s.charCreation.skillProfs);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);

  if (!classId || !backgroundId) {
    return (
      <section>
        <h2>{t('skills_title')}</h2>
        <p style={{ color: 'var(--color-text-muted)' }}>{t('skills_pick_class_first')}</p>
      </section>
    );
  }

  const klass = compendium.classes.find((c) => c.id === classId);
  const bg = compendium.backgrounds.find((b) => b.id === backgroundId);
  const bgSkills = new Set(bg?.skill_proficiencies ?? []);
  const klassData = klass ? readSkillProfData(klass) : {};
  const choose = klassData.choose ?? 0;
  const options = klassData.from ?? [];
  const chosenClassOnly = skillProfs.filter((s) => !bgSkills.has(s));

  function toggle(skillId: string) {
    if (bgSkills.has(skillId)) return;
    if (chosenClassOnly.includes(skillId)) {
      setDraftField(
        'skillProfs',
        skillProfs.filter((s) => s !== skillId),
      );
    } else if (chosenClassOnly.length < choose) {
      setDraftField('skillProfs', [...skillProfs, skillId]);
    }
  }

  return (
    <section>
      <h2>{t('skills_title')}</h2>
      <p>{t('skills_choose_n', { chosen: chosenClassOnly.length, total: choose })}</p>
      {bgSkills.size > 0 && (
        <p style={{ fontSize: 11, color: 'var(--color-text-muted)' }}>
          {t('skills_from_background', { list: [...bgSkills].join(', ') })}
        </p>
      )}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
        {options.map((s) => {
          const fromBg = bgSkills.has(s);
          const isChosen = skillProfs.includes(s) || fromBg;
          return (
            <button
              key={s}
              type="button"
              disabled={fromBg}
              className={`dm-wizard-card${isChosen ? ' is-selected' : ''}`}
              onClick={() => toggle(s)}
            >
              {t(`skill_${s}`, { defaultValue: s })}
              {fromBg ? ` (${t('locked')})` : ''}
            </button>
          );
        })}
      </div>
    </section>
  );
}
