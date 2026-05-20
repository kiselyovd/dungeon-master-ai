import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { backendUrl } from '../../api/client';
import { useStore } from '../../state/useStore';

function buildSeedPrompt(draft: {
  raceId: string | null;
  classId: string | null;
  alignment: string | null;
}): string {
  const parts: string[] = [];
  if (draft.raceId) parts.push(draft.raceId);
  if (draft.classId) parts.push(draft.classId);
  if (draft.alignment) parts.push(draft.alignment);
  parts.push('dark fantasy oil painting, by greg rutkowski');
  return parts.join(', ');
}

export interface PortraitTabProps {
  onOpenSettings?: () => void;
}

export function PortraitTab({ onOpenSettings }: PortraitTabProps) {
  const { t } = useTranslation('wizard');
  const draft = useStore((s) => s.charCreation);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const imageEnabled = useStore((s) => s.settings.imageEnabled);
  const imagePreset = useStore((s) => s.settings.imagePreset);
  const replicateApiKey = useStore((s) => s.settings.replicateApiKey);
  // "No provider" means cloud preset selected but no Replicate API key set.
  const noProviderConfigured = imagePreset === 'cloud' && replicateApiKey === null;
  const [prompt, setPrompt] = useState<string>(
    () => draft.portraitPrompt ?? buildSeedPrompt(draft),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function generate() {
    setBusy(true);
    setError(null);
    try {
      const url = await backendUrl('/image/generate');
      const resp = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          prompt,
          style_preset: 'character_portrait',
          scene_id: 'wizard',
        }),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = (await resp.json()) as { url?: string };
      if (data.url) {
        setDraftField('portraitUrl', data.url);
        setDraftField('portraitPrompt', prompt);
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  function skip() {
    setDraftField('portraitUrl', null);
    setDraftField('portraitPrompt', null);
  }

  if (!imageEnabled) {
    return (
      <section>
        <h2>{t('portrait_title')}</h2>
        <div className="dm-wizard-card" data-testid="portrait-image-disabled-card">
          <h3>{t('portrait_image_disabled_title')}</h3>
          <p>{t('portrait_image_disabled_body')}</p>
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            {onOpenSettings && (
              <button
                type="button"
                className="dm-wizard-btn-primary"
                data-testid="portrait-enable-btn"
                onClick={onOpenSettings}
              >
                {t('portrait_enable_now')}
              </button>
            )}
            <button type="button" className="dm-wizard-btn-secondary" onClick={skip}>
              {t('skip_portrait')}
            </button>
          </div>
        </div>
      </section>
    );
  }

  if (noProviderConfigured) {
    return (
      <section>
        <h2>{t('portrait_title')}</h2>
        <div className="dm-wizard-card" data-testid="portrait-image-no-provider-card">
          <h3>{t('portrait_image_no_provider_title')}</h3>
          <p>{t('portrait_image_no_provider_body')}</p>
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            {onOpenSettings && (
              <button
                type="button"
                className="dm-wizard-btn-primary"
                data-testid="portrait-configure-btn"
                onClick={onOpenSettings}
              >
                {t('portrait_configure_now')}
              </button>
            )}
            <button type="button" className="dm-wizard-btn-secondary" onClick={skip}>
              {t('skip_portrait')}
            </button>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section>
      <h2>{t('portrait_title')}</h2>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>
        <div
          className="dm-wizard-card"
          style={{
            aspectRatio: '1 / 1',
            cursor: 'default',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          {draft.portraitUrl ? (
            <img
              src={draft.portraitUrl}
              alt={t('portrait_alt')}
              style={{ width: '100%', height: '100%', objectFit: 'cover' }}
            />
          ) : (
            <span style={{ color: 'var(--color-text-muted)' }}>{t('portrait_empty')}</span>
          )}
        </div>
        <div>
          <label htmlFor="dm-portrait-prompt" className="dm-wizard-live-section-label">
            {t('portrait_prompt_label')}
          </label>
          <textarea
            id="dm-portrait-prompt"
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            style={{ width: '100%', minHeight: 200, marginTop: 4, padding: 8 }}
          />
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            <button
              type="button"
              className="dm-wizard-btn-primary"
              disabled={busy}
              onClick={() => {
                void generate();
              }}
            >
              {busy ? t('generating') : t('generate')}
            </button>
            <button type="button" className="dm-wizard-btn-secondary" onClick={skip}>
              {t('skip_portrait')}
            </button>
          </div>
          {error && (
            <p style={{ color: '#f06060', marginTop: 8, fontSize: 12 }}>
              {t('portrait_generation_failed', { error })}
            </p>
          )}
        </div>
      </div>
    </section>
  );
}
