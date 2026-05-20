/**
 * PresetStep - step 2 of 6 in the onboarding flow.
 *
 * Renders 5 preset cards as a keyboard-navigable radio group. Each card shows
 * the preset name, one-line description, configured-providers preview, and an
 * optional download-size hint. The local-only card displays a "Recommended"
 * badge. Selecting a card calls onPresetChange; Continue calls onNext.
 */

import { useTranslation } from 'react-i18next';
import { Icons } from '../../../ui/Icons';
import { PRESETS, type PresetId } from '../presets';

export interface PresetStepProps {
  titleId: string;
  preset: PresetId;
  onPresetChange: (preset: PresetId) => void;
  onBack: () => void;
  onNext: () => void;
}

const ICON_MAP = {
  Cpu: Icons.Cpu,
  Cloud: Icons.Cloud,
  Sparkle: Icons.Sparkle,
  Book: Icons.Book,
  Settings: Icons.Settings,
} as const;

export function PresetStep({ titleId, preset, onPresetChange, onBack, onNext }: PresetStepProps) {
  const { t } = useTranslation('onboarding');

  const handleGroupKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (!['ArrowDown', 'ArrowRight', 'ArrowUp', 'ArrowLeft'].includes(e.key)) return;
    e.preventDefault();
    const currentIndex = PRESETS.findIndex((p) => p.id === preset);
    let nextIndex: number;
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      nextIndex = (currentIndex + 1) % PRESETS.length;
    } else {
      nextIndex = (currentIndex - 1 + PRESETS.length) % PRESETS.length;
    }
    const nextPreset = PRESETS[nextIndex];
    if (nextPreset) {
      onPresetChange(nextPreset.id);
    }
  };

  return (
    <>
      <div className="dm-onboarding-tag">{t('preset_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('preset_step_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('preset_step_desc')}</p>

      <div
        className="dm-preset-cards"
        role="radiogroup"
        aria-labelledby={titleId}
        onKeyDown={handleGroupKeyDown}
      >
        {PRESETS.map((p) => {
          const IconComponent = ICON_MAP[p.icon];
          const isSelected = preset === p.id;
          return (
            // biome-ignore lint/a11y/useSemanticElements: card layout requires button; aria role="radio" applied
            <button
              key={p.id}
              type="button"
              role="radio"
              aria-checked={isSelected}
              tabIndex={isSelected ? 0 : -1}
              className={`dm-preset-card${isSelected ? ' is-selected' : ''}`}
              onClick={() => {
                onPresetChange(p.id);
              }}
            >
              <div className="dm-preset-card-header">
                <div className="dm-preset-card-icon">
                  <IconComponent size={18} />
                </div>
                <span className="dm-preset-card-name">{t(p.labelKey)}</span>
                {p.recommended && (
                  <span className="dm-preset-card-badge">{t('preset_recommended')}</span>
                )}
              </div>
              <p className="dm-preset-card-desc">{t(p.descKey)}</p>
              <span className="dm-preset-card-providers">{t(p.providersKey)}</span>
              {p.downloadKey && <span className="dm-preset-card-download">{t(p.downloadKey)}</span>}
            </button>
          );
        })}
      </div>

      <div className="dm-onboarding-actions">
        <button
          type="button"
          className="dm-onboarding-btn dm-onboarding-btn-secondary"
          onClick={onBack}
        >
          <Icons.ChevronLeft size={14} />
          {t('back')}
        </button>
        <button
          type="button"
          className="dm-onboarding-btn dm-onboarding-btn-primary"
          onClick={onNext}
        >
          {t('next')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}
