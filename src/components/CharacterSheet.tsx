/**
 * CharacterSheet (M5 P2.14).
 *
 * Modal that mirrors the design jsx in
 * `docs/superpowers/design/components/onboarding.jsx` (the lower half,
 * `function CharacterSheet`). Reads the full PC slice and renders:
 *   - Header: portrait initial, name, race - class - level, background.
 *   - XP bar.
 *   - Combat block + Saving throws (left column).
 *   - Ability grid + Skills + Inventory (right column).
 *
 * The modal is presentational - it never mutates state. Combat/journal
 * hooks elsewhere update the slice (e.g. on damage); the sheet just shows
 * the current snapshot.
 *
 * Empty state: when no character has been created yet (`pc.name === null`)
 * the sheet renders a single "Go to onboarding" CTA that flips the
 * `onboarding.completed` flag back to `false` so the wizard pops up.
 */

import { type ReactElement, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import portraitCleric from '../assets/char-portrait-cleric.png';
import portraitFighter from '../assets/char-portrait-fighter.png';
import portraitPaladin from '../assets/char-portrait-paladin.png';
import portraitRogue from '../assets/char-portrait-rogue.png';
import portraitWizard from '../assets/char-portrait-wizard.png';
import {
  type AbilityScores,
  abilityMod,
  type SavingThrowProf,
  type SkillProf,
  savingThrowMod,
  skillMod,
} from '../state/pc';
import { useStore } from '../state/useStore';
import type { IconName } from '../ui/Icons';
import { Icons } from '../ui/Icons';

const CLASS_PORTRAIT: Record<string, string> = {
  fighter: portraitFighter,
  wizard: portraitWizard,
  rogue: portraitRogue,
  cleric: portraitCleric,
  paladin: portraitPaladin,
};

interface CharacterSheetProps {
  open: boolean;
  onClose: () => void;
}

type AbilityKey = keyof AbilityScores;

const ABILITY_ORDER: readonly AbilityKey[] = ['str', 'dex', 'con', 'int', 'wis', 'cha'];

const SAVE_ABILITIES: readonly AbilityKey[] = ['str', 'dex', 'con', 'int', 'wis', 'cha'];

interface SkillRow {
  key: keyof SkillProf;
  ability: AbilityKey;
}

const SKILL_ROWS: readonly SkillRow[] = [
  { key: 'acrobatics', ability: 'dex' },
  { key: 'athletics', ability: 'str' },
  { key: 'arcana', ability: 'int' },
  { key: 'deception', ability: 'cha' },
  { key: 'history', ability: 'int' },
  { key: 'insight', ability: 'wis' },
  { key: 'intimidation', ability: 'cha' },
  { key: 'investigation', ability: 'int' },
  { key: 'perception', ability: 'wis' },
  { key: 'persuasion', ability: 'cha' },
  { key: 'stealth', ability: 'dex' },
  { key: 'survival', ability: 'wis' },
];

function formatMod(value: number): string {
  return value >= 0 ? `+${value}` : `${value}`;
}

type IconComponent = (props: { size?: number }) => ReactElement;

function inventoryIcon(slug: string | undefined): IconComponent {
  if (!slug) return Icons.Tag as IconComponent;
  const map: Record<string, IconName> = {
    sword: 'Sword',
    bow: 'Bow',
    shield: 'Shield',
    potion: 'Potion',
    coin: 'Coin',
    scroll: 'Scroll',
  };
  const name = map[slug.toLowerCase()];
  if (name && name in Icons) {
    return Icons[name] as IconComponent;
  }
  return Icons.Tag as IconComponent;
}

export function CharacterSheet({ open, onClose }: CharacterSheetProps) {
  const { t } = useTranslation('character');
  const pc = useStore((s) => s.pc);
  const resetOnboarding = useStore((s) => s.onboarding.reset);

  // Escape closes the modal.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  // Pre-compute mods so the JSX stays tidy.
  const abilityMods = useMemo(() => {
    const result: Record<AbilityKey, number> = {
      str: abilityMod(pc.abilities.str),
      dex: abilityMod(pc.abilities.dex),
      con: abilityMod(pc.abilities.con),
      int: abilityMod(pc.abilities.int),
      wis: abilityMod(pc.abilities.wis),
      cha: abilityMod(pc.abilities.cha),
    };
    return result;
  }, [pc.abilities]);

  if (!open) return null;

  // Empty state: no character has been created yet.
  if (pc.name === null) {
    return (
      <div
        className="dm-modal-backdrop dm-char-backdrop"
        role="dialog"
        aria-modal="true"
        aria-label={t('title')}
        onClick={onClose}
        onKeyDown={(e) => {
          if (e.key === 'Escape') onClose();
        }}
      >
        {/* biome-ignore lint/a11y/noStaticElementInteractions: stopPropagation only - keyboard a11y is on the parent overlay */}
        {/* biome-ignore lint/a11y/useKeyWithClickEvents: inner card only suppresses click bubbling; the overlay above owns Escape */}
        <div className="dm-char dm-vignette" onClick={(e) => e.stopPropagation()}>
          <div className="dm-modal-header">
            <h2 className="dm-display dm-modal-title">{t('title')}</h2>
            <button
              type="button"
              className="dm-btn-icon"
              onClick={onClose}
              aria-label={t('close_aria')}
            >
              <Icons.X size={14} />
            </button>
          </div>
          <div className="dm-char-empty">
            <h3 className="dm-char-empty-title">{t('empty_title')}</h3>
            <p className="dm-char-empty-desc">{t('empty_desc')}</p>
            <button
              type="button"
              className="dm-onboarding-btn dm-onboarding-btn-primary"
              onClick={() => {
                resetOnboarding();
                onClose();
              }}
            >
              <Icons.Sparkle size={14} />
              {t('empty_action')}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Resolved (non-null) character snapshot - safe to render the full sheet.
  const portraitInitial = pc.name.charAt(0).toUpperCase();
  const portraitSrc = pc.heroClass !== null ? (CLASS_PORTRAIT[pc.heroClass] ?? null) : null;
  const className = pc.heroClass !== null ? t(`class_${pc.heroClass}` as 'class_fighter') : '';
  const meta =
    pc.subclass !== null
      ? t('meta_race_subclass_level', {
          race: pc.race ?? '',
          className,
          subclass: pc.subclass,
          level: pc.level,
        })
      : t('meta_race_class_level', {
          race: pc.race ?? '',
          className,
          level: pc.level,
        });
  const bgLine = t('meta_background_alignment', {
    background: pc.background ?? '',
    alignment: pc.alignment ?? '',
  });

  const xpPct =
    pc.experienceNext > 0
      ? Math.max(0, Math.min(100, (pc.experience / pc.experienceNext) * 100))
      : 0;
  const hpPct = pc.hpMax > 0 ? Math.max(0, Math.min(100, (pc.hp / pc.hpMax) * 100)) : 0;
  const hpLow = hpPct < 25;

  return (
    <div
      className="dm-modal-backdrop dm-char-backdrop"
      role="dialog"
      aria-modal="true"
      aria-label={t('title')}
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === 'Escape') onClose();
      }}
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: stopPropagation only - keyboard a11y is on the parent overlay */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: inner card only suppresses click bubbling; the overlay above owns Escape */}
      <div className="dm-char dm-vignette" onClick={(e) => e.stopPropagation()}>
        <div className="dm-modal-header">
          <h2 className="dm-display dm-modal-title">{t('title')}</h2>
          <button
            type="button"
            className="dm-btn-icon"
            onClick={onClose}
            aria-label={t('close_aria')}
          >
            <Icons.X size={14} />
          </button>
        </div>

        <div className="dm-char-header">
          <div className="dm-char-portrait" aria-hidden="true">
            {portraitSrc !== null ? (
              <img src={portraitSrc} alt="" className="dm-char-portrait-art" />
            ) : (
              portraitInitial
            )}
          </div>
          <div className="dm-char-info">
            <h2 className="dm-char-name">{pc.name}</h2>
            <div className="dm-char-meta">{meta}</div>
            <div className="dm-char-meta dm-char-meta-soft">{bgLine}</div>
          </div>
          <div className="dm-char-xp">
            <span className="dm-char-xp-label">{t('experience_label')}</span>
            <div className="dm-char-xp-bar">
              <div className="dm-char-xp-fill" style={{ width: `${xpPct}%` }} />
            </div>
            <span className="dm-char-xp-text dm-mono">
              {t('experience_value', { xp: pc.experience, next: pc.experienceNext })}
            </span>
          </div>
        </div>

        <div className="dm-char-body">
          {/* Left column: Combat + Saving Throws */}
          <div>
            <div className="dm-char-section">
              <h3 className="dm-char-section-title">
                <Icons.Heart size={11} /> {t('section_combat')}
              </h3>
              <div className="dm-stat-row">
                <span>{t('stat_hp')}</span>
                <span className={`dm-mono${hpLow ? ' dm-stat-hp-low' : ''}`}>
                  {pc.hp} / {pc.hpMax}
                </span>
              </div>
              <div className="dm-char-hpbar" aria-hidden="true">
                <div
                  className={`dm-char-hpbar-fill${hpLow ? ' is-low' : ''}`}
                  style={{ width: `${hpPct}%` }}
                />
              </div>
              <div className="dm-stat-row">
                <span>{t('stat_ac')}</span>
                <span className="dm-mono">{pc.ac}</span>
              </div>
              <div className="dm-stat-row">
                <span>{t('stat_initiative')}</span>
                <span className="dm-mono">{formatMod(pc.initiative)}</span>
              </div>
              <div className="dm-stat-row">
                <span>{t('stat_speed')}</span>
                <span className="dm-mono">{t('stat_speed_value', { ft: pc.speedFt })}</span>
              </div>
              <div className="dm-stat-row">
                <span>{t('stat_proficiency')}</span>
                <span className="dm-mono">{formatMod(pc.proficiencyBonus)}</span>
              </div>
            </div>

            <div className="dm-char-section">
              <h3 className="dm-char-section-title">
                <Icons.ShieldHalf size={11} /> {t('section_saving_throws')}
              </h3>
              {SAVE_ABILITIES.map((a) => {
                const prof = pc.savingThrowProfs[a as keyof SavingThrowProf] === true;
                const mod = savingThrowMod(pc.abilities[a], prof, pc.proficiencyBonus);
                return (
                  <div key={a} className="dm-stat-row">
                    <span>{t(`save_${a}` as 'save_str')}</span>
                    <span className="dm-mono">
                      {formatMod(mod)}
                      {prof ? ' ⚪' : ''}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right column: Abilities + Skills + Inventory */}
          <div>
            <div className="dm-char-section">
              <h3 className="dm-char-section-title">
                <Icons.Star size={11} /> {t('section_abilities')}
              </h3>
              <div className="dm-ability-grid">
                {ABILITY_ORDER.map((a) => (
                  <div key={a} className="dm-ability">
                    <span className="dm-ability-name">{t(`ability_${a}` as 'ability_str')}</span>
                    <span className="dm-ability-mod dm-mono">{formatMod(abilityMods[a])}</span>
                    <span className="dm-ability-score dm-mono">{pc.abilities[a]}</span>
                  </div>
                ))}
              </div>
            </div>

            <div className="dm-char-section">
              <h3 className="dm-char-section-title">
                <Icons.Crosshair size={11} /> {t('section_skills')}
              </h3>
              <div className="dm-skills">
                {SKILL_ROWS.map(({ key, ability }) => {
                  const prof = pc.skillProfs[key] === true;
                  const mod = skillMod(pc.abilities[ability], prof, pc.proficiencyBonus);
                  return (
                    <div key={key} className={`dm-skill${prof ? ' is-prof' : ''}`}>
                      <span className="dm-skill-name">
                        <span className="dm-skill-prof" aria-hidden="true" />
                        {t(`skill_${key}` as 'skill_acrobatics')}
                      </span>
                      <span className="dm-mono">{formatMod(mod)}</span>
                    </div>
                  );
                })}
              </div>
            </div>

            <div className="dm-char-section">
              <h3 className="dm-char-section-title">
                <Icons.Potion size={11} /> {t('section_inventory')}
              </h3>
              <div className="dm-inv-list">
                {pc.inventory.map((item) => {
                  const Icon = inventoryIcon(item.icon);
                  return (
                    <div key={item.id} className="dm-inv-item">
                      <Icon size={14} />
                      <span className="dm-inv-item-name">{item.name}</span>
                      <span className="dm-inv-item-count dm-mono">x{item.count}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
