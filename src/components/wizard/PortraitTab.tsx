import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { backendUrl } from '../../api/client';
import { parseMediaReference } from '../../api/contracts/mediaAsset';
import { readMedia, uploadPortrait } from '../../api/mediaAssets';
import { portraitAssetUrl, portraitFromBackendAsset } from '../../assets/visualCatalog';
import { useMediaImage } from '../../features/media/useMediaImage';
import { legacyPortraitUrl } from '../../lib/heroPortraits';
import { useStore } from '../../state/useStore';
import { PortraitLibrary } from './PortraitLibrary';

function buildSeedPrompt(draft: {
  raceId: string | null;
  classId: string | null;
  alignment: string | null;
}): string {
  const parts: string[] = [];
  if (draft.raceId) parts.push(draft.raceId);
  if (draft.classId) parts.push(draft.classId);
  if (draft.alignment) parts.push(draft.alignment);
  parts.push(
    'classic fantasy oil and gouache, expressive natural face, warm practical light, rich worn materials',
  );
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
  const generation = useRef(0);
  const fileInput = useRef<HTMLInputElement>(null);
  const media = useMediaImage(
    draft.portraitMediaRef,
    draft.mediaDraftId ? { kind: 'draft', draft_id: draft.mediaDraftId } : null,
    'portrait',
  );
  useEffect(
    () => () => {
      generation.current++;
    },
    [],
  );
  function draftIdentity() {
    const existing = useStore.getState().charCreation.mediaDraftId;
    if (existing) return existing;
    const id = crypto.randomUUID();
    setDraftField('mediaDraftId', id);
    return id;
  }
  async function chooseFile(file: File) {
    if (busy) return;
    const id = draftIdentity();
    const sequence = ++generation.current;
    setBusy(true);
    setError(null);
    try {
      const reference = await uploadPortrait({ kind: 'draft', draft_id: id }, file);
      if (typeof createImageBitmap === 'function') {
        const bitmap = await createImageBitmap(file);
        const valid =
          bitmap.width === reference.descriptor.width &&
          bitmap.height === reference.descriptor.height;
        bitmap.close();
        if (!valid) throw new Error('media_asset_invalid');
      }
      if (generation.current !== sequence || useStore.getState().charCreation.mediaDraftId !== id)
        return;
      setDraftField('portraitMediaRef', reference);
      setDraftField('portraitAssetId', null);
      setDraftField('portraitUrl', null);
    } catch (e) {
      if (generation.current === sequence)
        setError(e instanceof Error ? e.message : 'media_asset_invalid');
    } finally {
      if (generation.current === sequence) setBusy(false);
    }
  }
  const library = (
    <>
      <PortraitLibrary
        heroClass={draft.classId}
        selected={draft.portraitAssetId}
        disabled={busy}
        onSelect={(id) => {
          generation.current++;
          setDraftField('portraitMediaRef', null);
          setDraftField('portraitAssetId', id);
          setDraftField('portraitUrl', null);
        }}
      />
      <input
        ref={fileInput}
        type="file"
        accept="image/png,image/jpeg,image/webp"
        hidden
        aria-label={t('portrait_upload')}
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = '';
          if (file) void chooseFile(file);
        }}
      />
      <button
        type="button"
        className="dm-wizard-btn-secondary"
        disabled={busy}
        onClick={() => fileInput.current?.click()}
      >
        {t('portrait_upload')}
      </button>
      {draft.portraitMediaRef && (
        <div className="dm-wizard-card">
          {media.url ? (
            <img
              src={media.url}
              alt={t('portrait_alt')}
              style={{ width: 160, height: 160, objectFit: 'cover' }}
            />
          ) : (
            <span>{t(media.error ? 'portrait_unavailable' : 'generating')}</span>
          )}
          {media.error && (
            <button type="button" className="dm-wizard-btn-secondary" onClick={media.retry}>
              {t('portrait_retry')}
            </button>
          )}
        </div>
      )}
      {error && (
        <p role="alert" style={{ color: 'var(--color-danger)' }}>
          {t(
            error === 'media_asset_too_large'
              ? 'portrait_too_large'
              : error.startsWith('media_asset_')
                ? 'portrait_upload_failed'
                : 'portrait_unavailable',
          )}
        </p>
      )}
    </>
  );
  const portrait = draft.portraitMediaRef
    ? media.url
    : draft.portraitAssetId
      ? portraitAssetUrl(draft.portraitAssetId)
      : legacyPortraitUrl(draft.portraitUrl);

  async function generate() {
    if (busy) return;
    const id = draftIdentity();
    const sequence = ++generation.current;
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
          asset_scope: { kind: 'draft', draft_id: id },
        }),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = (await resp.json()) as {
        url?: string;
        source?: string;
        asset_id?: string;
        media_ref?: unknown;
      };
      if (generation.current !== sequence || useStore.getState().charCreation.mediaDraftId !== id)
        return;
      if (data.source === 'bundled') {
        const bundled =
          typeof data.asset_id === 'string' ? portraitFromBackendAsset(data.asset_id) : null;
        if (!bundled) throw new Error('portrait_unavailable');
        setDraftField('portraitAssetId', bundled.id);
        setDraftField('portraitMediaRef', null);
        setDraftField('portraitUrl', null);
        setDraftField('portraitPrompt', prompt);
      } else if (data.media_ref) {
        const reference = parseMediaReference(data.media_ref);
        if (
          reference.source !== 'owned' ||
          reference.category !== 'portrait' ||
          reference.origin !== 'generated'
        )
          throw new Error('portrait_unavailable');
        await readMedia(reference, { kind: 'draft', draft_id: id });
        if (generation.current !== sequence || useStore.getState().charCreation.mediaDraftId !== id)
          return;
        setDraftField('portraitMediaRef', reference);
        setDraftField('portraitAssetId', null);
        setDraftField('portraitUrl', null);
        setDraftField('portraitPrompt', prompt);
      } else if (data.url) {
        // Compatibility server: retain the old choice until durable registration succeeds.
        const { decodeLegacyMedia } = await import('../../api/mediaAssets');
        const decoded = decodeLegacyMedia(data.url);
        if (!decoded) throw new Error('portrait_unavailable');
        const reference = await uploadPortrait(
          { kind: 'draft', draft_id: id },
          new Blob([new Uint8Array(decoded.bytes).buffer], { type: decoded.mimeType }),
          true,
        );
        if (generation.current !== sequence || useStore.getState().charCreation.mediaDraftId !== id)
          return;
        setDraftField('portraitAssetId', null);
        setDraftField('portraitMediaRef', reference);
        setDraftField('portraitUrl', null);
        setDraftField('portraitPrompt', prompt);
      }
    } catch (e) {
      if (generation.current === sequence) setError((e as Error).message);
    } finally {
      if (generation.current === sequence) setBusy(false);
    }
  }

  function skip() {
    generation.current++;
    setBusy(false);
    setDraftField('portraitMediaRef', null);
    setDraftField('portraitAssetId', null);
    setDraftField('portraitUrl', null);
    setDraftField('portraitPrompt', null);
  }

  if (!imageEnabled) {
    return (
      <section>
        <h2>{t('portrait_title')}</h2>
        {library}
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
        {library}
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
      {library}
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
          {portrait ? (
            <img
              src={portrait}
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
        </div>
      </div>
    </section>
  );
}
