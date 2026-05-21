import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { startLocalRuntimes, stopLocalRuntimes } from '../../api/localRuntime';
import { useLocalRuntimeStatus } from '../../hooks/useLocalRuntimeStatus';
import { useModelDownload } from '../../hooks/useModelDownload';
import type { ModelId, VramStrategy } from '../../state/localMode';
import { useStore } from '../../state/useStore';
import { Field } from '../../ui/Field';
import { CustomHfRepoModal } from '../CustomHfRepoModal';
import { ModelDownloadCard } from '../ModelDownloadCard';
import { RuntimeStatusPill } from '../RuntimeStatusPill';
import styles from '../SettingsForm.module.css';
import { ModelSelector as LocalLlmModelSelector } from './local-llm/ModelSelector';

/**
 * Map between the localMode slice `ModelId` enum (snake_case) and the wire
 * id used by the new local-llm manifest endpoint (e.g. `qwen3.5-4b`). Lets the
 * Settings -> Local LLM picker keep `localMode.selectedLlm` in sync with the
 * value the backend manifest understands.
 *
 * Custom HF ids fall through (return null) - selection there flows through the
 * separate `customLlmOverride` slot, not the manifest picker.
 */
const LOCAL_MODEL_WIRE_ID: Partial<Record<ModelId, string>> = {
  qwen3_5_0_8b: 'qwen3.5-0.8b',
  qwen3_5_2b: 'qwen3.5-2b',
  qwen3_5_4b: 'qwen3.5-4b',
  qwen3_5_9b: 'qwen3.5-9b',
};
const WIRE_ID_TO_LOCAL_MODEL: Record<string, ModelId> = Object.fromEntries(
  Object.entries(LOCAL_MODEL_WIRE_ID).map(([k, v]) => [v as string, k as ModelId]),
);

interface LocalLlmEntry {
  id: ModelId;
  name: string;
  size: number;
  vram: number;
  warn?: string;
}

const LOCAL_LLMS: readonly LocalLlmEntry[] = [
  { id: 'qwen3_5_0_8b', name: 'Qwen3.5-0.8B Q4_K_M', size: 600e6, vram: 900e6 },
  { id: 'qwen3_5_2b', name: 'Qwen3.5-2B Q4_K_M', size: 1.5e9, vram: 2.0e9 },
  { id: 'qwen3_5_4b', name: 'Qwen3.5-4B Q4_K_M', size: 3.0e9, vram: 2.5e9 },
  {
    id: 'qwen3_5_9b',
    name: 'Qwen3.5-9B Q4_K_M',
    size: 6.5e9,
    vram: 5.5e9,
    warn: 'requires VRAM swap with image-gen',
  },
];

const RUNTIME_RESET_DELAY_MS = 3500;
type RuntimeActionStatus = 'idle' | 'pending' | 'error';

/**
 * Standalone Settings -> Local LLM tab (D8).
 *
 * Mirrors the LocalModeModal (Ctrl+Shift+M) so the embedded local-mistralrs
 * provider can be configured without leaving Settings: pick a Qwen variant,
 * choose a VRAM strategy, and start/stop the LLM + image runtimes. The shared
 * state lives in the localMode slice so both surfaces stay in sync.
 *
 * Promoted out of the Chat tab in D8: this panel is now ALWAYS visible as its
 * own tab, regardless of which chat provider is active.
 */
export function LocalLlmTab() {
  const { t } = useTranslation('settings');
  const { t: tLocal } = useTranslation('local_mode');
  const lm = useStore((s) => s.localMode);
  const [customModalOpen, setCustomModalOpen] = useState(false);
  // Poll runtime status so the pills + the toWireConfig port lookup reflect
  // the current sidecar state. While the Local LLM panel is mounted we always
  // poll - the user is actively configuring it, so the original `enabled`
  // gate would be a confusing extra hoop.
  useLocalRuntimeStatus(true);

  const [startStatus, setStartStatus] = useState<RuntimeActionStatus>('idle');
  const [stopStatus, setStopStatus] = useState<RuntimeActionStatus>('idle');
  const startResetRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const stopResetRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearStartReset = useCallback(() => {
    if (startResetRef.current !== null) {
      clearTimeout(startResetRef.current);
      startResetRef.current = null;
    }
  }, []);

  const clearStopReset = useCallback(() => {
    if (stopResetRef.current !== null) {
      clearTimeout(stopResetRef.current);
      stopResetRef.current = null;
    }
  }, []);

  useEffect(
    () => () => {
      clearStartReset();
      clearStopReset();
    },
    [clearStartReset, clearStopReset],
  );

  const handleStart = useCallback(async () => {
    clearStartReset();
    setStartStatus('pending');
    try {
      await startLocalRuntimes();
      setStartStatus('idle');
    } catch {
      setStartStatus('error');
      startResetRef.current = setTimeout(() => {
        setStartStatus('idle');
        startResetRef.current = null;
      }, RUNTIME_RESET_DELAY_MS);
    }
  }, [clearStartReset]);

  const handleStop = useCallback(async () => {
    clearStopReset();
    setStopStatus('pending');
    try {
      await stopLocalRuntimes();
      setStopStatus('idle');
    } catch {
      setStopStatus('error');
      stopResetRef.current = setTimeout(() => {
        setStopStatus('idle');
        stopResetRef.current = null;
      }, RUNTIME_RESET_DELAY_MS);
    }
  }, [clearStopReset]);

  // M9-DM Task 14: drop the new manifest-driven ModelSelector container in as
  // a NEW section above the existing per-card controls. The legacy LOCAL_LLMS
  // cards stay for one more commit; consolidation happens in a follow-up task
  // once the HF search lands in Tasks 15-19.
  const activeWireId = LOCAL_MODEL_WIRE_ID[lm.selectedLlm] ?? null;
  const onActiveLocalChange = (wireId: string) => {
    const mapped = WIRE_ID_TO_LOCAL_MODEL[wireId];
    if (mapped) lm.selectModel(mapped);
    // TODO(M9-DM): forward custom HF ids (not in LOCAL_MODEL_WIRE_ID) to the
    // override slot here once HF search persists user manifests (Task 19).
  };

  return (
    <div className={styles.localFields}>
      <div className={styles.localHint}>{t('local_runtime_hint')}</div>

      {/* TODO(M9-DM): consolidate the LOCAL_LLMS cards into this picker once
          the download wiring lands in Task 19. */}
      <LocalLlmModelSelector
        activeId={activeWireId}
        onActiveChange={onActiveLocalChange}
        // TODO(M9-DM): plumb session.agentTurnInFlight once the session slice
        // exposes it; until then assume the user is not mid-turn (false).
        agentTurnInFlight={false}
      />

      <h3 className={styles.localHeading}>{tLocal('llm_models')}</h3>
      {LOCAL_LLMS.map((m) => (
        <LocalModelCard key={m.id} entry={m} isLlm />
      ))}

      <div className={styles.localCustomBlock}>
        {lm.customLlmOverride ? (
          <div className={styles.localCustomRow}>
            <span className={styles.localCustomLabel}>
              {lm.customLlmOverride.hf_repo}/{lm.customLlmOverride.gguf_filename}
              {lm.customLlmOverride.mmproj_filename
                ? ` (+${lm.customLlmOverride.mmproj_filename})`
                : ''}
            </span>
            <button type="button" onClick={() => lm.setCustomLlmOverride(null)}>
              {t('custom_modal_cancel')}
            </button>
          </div>
        ) : (
          <p className={styles.localCustomHelper}>{t('model_selector_custom_helper')}</p>
        )}
        <button
          type="button"
          className={styles.localCustomAddButton}
          onClick={() => setCustomModalOpen(true)}
        >
          {t('model_selector_custom_add_button')}
        </button>
      </div>

      <CustomHfRepoModal
        open={customModalOpen}
        onClose={() => setCustomModalOpen(false)}
        onSave={(input) => {
          lm.setCustomLlmOverride(input);
          setCustomModalOpen(false);
        }}
      />

      <h3 className={styles.localHeading}>{tLocal('image_model')}</h3>
      <LocalModelCard
        entry={{ id: 'sdxl_turbo', name: 'SDXL-Turbo (fp16)', size: 7e9 }}
        isLlm={false}
      />

      <Field label={tLocal('vram_strategy')}>
        {({ id }) => (
          <select
            id={id}
            value={lm.vramStrategy}
            onChange={(e) => lm.setVramStrategy(e.target.value as VramStrategy)}
            className={styles.fullWidth}
          >
            <option value="auto-swap">{tLocal('strategy_auto_swap')}</option>
            <option value="keep-both-loaded">{tLocal('strategy_keep_both')}</option>
            <option value="disable-image-gen">{tLocal('strategy_disable_image')}</option>
          </select>
        )}
      </Field>

      <h3 className={styles.localHeading}>{t('local_runtime_section')}</h3>
      <div className={styles.localRuntimeRow}>
        <button
          type="button"
          disabled={startStatus === 'pending'}
          data-status={startStatus}
          onClick={() => {
            void handleStart();
          }}
        >
          {startStatus === 'pending' ? tLocal('runtime_starting') : tLocal('start_runtimes')}
        </button>
        {startStatus === 'error' && (
          <span role="alert" className={styles.localErrorChip}>
            {tLocal('runtime_start_error')}
          </span>
        )}
        <button
          type="button"
          disabled={stopStatus === 'pending'}
          data-status={stopStatus}
          onClick={() => {
            void handleStop();
          }}
        >
          {stopStatus === 'pending' ? tLocal('runtime_stopping') : tLocal('stop_runtimes')}
        </button>
        {stopStatus === 'error' && (
          <span role="alert" className={styles.localErrorChip}>
            {tLocal('runtime_stop_error')}
          </span>
        )}
        <RuntimeStatusPill label={tLocal('runtime_pill_llm')} state={lm.runtime.llm} />
        <RuntimeStatusPill label={tLocal('runtime_pill_image')} state={lm.runtime.image} />
      </div>
      {lm.runtime.llm.state !== 'ready' && (
        <div className={styles.localHint}>{t('local_runtime_not_ready')}</div>
      )}
    </div>
  );
}

/**
 * Inline ModelDownloadCard binding that wires a manifest entry to the
 * download hook + the localMode slice. Mirrors the helper inside
 * LocalModeModal so both surfaces share the same selection semantics.
 *
 * Intentionally duplicated (D8 atomic choice - shared-module extraction
 * deferred to a later polish pass). The surviving identical twin is
 * `ModelCard` in `src/components/LocalModeModal.tsx`.
 */
function LocalModelCard({
  entry,
  isLlm,
}: {
  entry: { id: ModelId; name: string; size: number; vram?: number; warn?: string };
  isLlm: boolean;
}) {
  const lm = useStore((s) => s.localMode);
  const dl = useModelDownload(entry.id);
  return (
    <ModelDownloadCard
      modelId={entry.id}
      displayName={entry.name}
      sizeBytes={entry.size}
      vramBytes={entry.vram}
      vramWarning={entry.warn}
      state={lm.downloads[entry.id]}
      active={isLlm && lm.selectedLlm === entry.id}
      onSelect={() => isLlm && lm.selectModel(entry.id)}
      onDownload={() => {
        void dl.start();
      }}
      onDelete={() => {
        void dl.cancel();
      }}
    />
  );
}
