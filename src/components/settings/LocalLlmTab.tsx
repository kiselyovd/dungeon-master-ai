import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  fetchLocalRuntimeStatus,
  persistLocalModeConfig,
  startLocalRuntimes,
  stopLocalRuntimes,
} from '../../api/localRuntime';
import { useLocalRuntimeStatus } from '../../hooks/useLocalRuntimeStatus';
import { useModelDownload } from '../../hooks/useModelDownload';
import { useLocalLlmStore } from '../../state/localLlm';
import {
  type ModelId,
  selectedLocalModelIsRunning,
  type VramStrategy,
} from '../../state/localMode';
import { useStore } from '../../state/useStore';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import { Icons } from '../../ui/Icons';
import { Select } from '../../ui/Select';
import { ModelDownloadCard } from '../ModelDownloadCard';
import { RuntimeStatusPill } from '../RuntimeStatusPill';
import styles from '../SettingsForm.module.css';
import { CollapsibleCard } from './local-llm/CollapsibleCard';
import { ModelSelector as LocalLlmModelSelector } from './local-llm/ModelSelector';

/**
 * Map between the localMode slice `ModelId` enum (snake_case) and the wire
 * id used by the new local-llm manifest endpoint (e.g. `qwen3.5-4b`). Lets the
 * Settings -> Local LLM picker keep `localMode.selectedLlm` in sync with the
 * value the backend manifest understands.
 *
 * Custom HF ids fall through (return null) - those are added through the HF
 * search inside `LocalLlmModelSelector`, which persists them to the user
 * manifest the backend picker reads.
 */
const LOCAL_MODEL_WIRE_ID: Partial<Record<ModelId, string>> = {
  qwen3_8b: 'qwen3-8b',
  gemma4_e2b: 'gemma-4-e2b',
  gemma4_e4b: 'gemma-4-e4b',
  qwen3_5_0_8b: 'qwen3.5-0.8b',
  qwen3_5_2b: 'qwen3.5-2b',
  qwen3_5_4b: 'qwen3.5-4b',
  qwen3_5_9b: 'qwen3.5-9b',
};
const WIRE_ID_TO_LOCAL_MODEL: Record<string, ModelId> = Object.fromEntries(
  Object.entries(LOCAL_MODEL_WIRE_ID).map(([k, v]) => [v as string, k as ModelId]),
);

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
export function LocalLlmTab({ agentTurnInFlight = false }: { agentTurnInFlight?: boolean } = {}) {
  const { t } = useTranslation('settings');
  const { t: tLocal } = useTranslation('local_mode');
  const lm = useStore((s) => s.localMode);
  const soloBusy = useStore((s) => s.chat.isStreaming);
  const isStreaming = soloBusy || agentTurnInFlight;
  const installedIds = useLocalLlmStore((s) => s.installedIds);
  const catalog = useLocalLlmStore((s) => s.system);
  const catalogError = useLocalLlmStore((s) => s.error);
  const catalogLoading = useLocalLlmStore((s) => s.loading);
  const [checking, setChecking] = useState(false);
  // Poll runtime status so the pills + the toWireConfig port lookup reflect
  // the current sidecar state. While the Local LLM panel is mounted we always
  // poll - the user is actively configuring it, so the original `enabled`
  // gate would be a confusing extra hoop.
  useLocalRuntimeStatus(true);

  const [startStatus, setStartStatus] = useState<RuntimeActionStatus>('idle');
  const [stopStatus, setStopStatus] = useState<RuntimeActionStatus>('idle');
  // Capture the backend's actual failure reason (e.g. a 0-byte/missing sidecar
  // binary) so "runtime won't start" is debuggable instead of a mute chip.
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
    if (isStreaming || startStatus === 'pending') return;
    clearStartReset();
    setStartStatus('pending');
    lm.markRuntimeStatusUnknown();
    try {
      if (lm.runtime.llm.state === 'ready' || lm.runtime.llm.state === 'failed')
        await stopLocalRuntimes();
      await persistLocalModeConfig({
        context_window: lm.contextWindow,
        selected_llm: lm.selectedLlm,
        vram_strategy: lm.vramStrategy,
      });
      await startLocalRuntimes();
      setStartStatus('idle');
    } catch {
      console.warn('[LocalLlmTab]', { code: 'runtime_start_failed', modelId: lm.selectedLlm });
      setStartStatus('error');
      startResetRef.current = setTimeout(() => {
        setStartStatus('idle');
        startResetRef.current = null;
      }, RUNTIME_RESET_DELAY_MS);
    }
  }, [
    clearStartReset,
    lm.selectedLlm,
    lm.vramStrategy,
    lm.contextWindow,
    lm.runtime.llm.state,
    lm.markRuntimeStatusUnknown,
    isStreaming,
    startStatus,
  ]);

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

  // The manifest-driven selector is the single LLM surface: active picker,
  // Manage Downloads (with the HF token row), and HF search that persists
  // user-added models to the manifest the backend picker reads. The legacy
  // LOCAL_LLMS cards + CustomHfRepoModal were removed in M11 Batch D.
  const activeWireId = LOCAL_MODEL_WIRE_ID[lm.selectedLlm] ?? null;
  const installed = activeWireId !== null && installedIds.has(activeWireId);
  const selectedName =
    catalog.find((entry) => entry.id === activeWireId)?.display_name ?? t('model_not_selected');
  async function checkStatus() {
    if (checking) return;
    setChecking(true);
    try {
      lm.setRuntimeStatus(await fetchLocalRuntimeStatus());
    } catch {
      lm.markRuntimeStatusUnknown();
    } finally {
      setChecking(false);
    }
  }
  const onActiveLocalChange = (wireId: string) => {
    const mapped = WIRE_ID_TO_LOCAL_MODEL[wireId];
    if (mapped) lm.selectModel(mapped);
    else setStartStatus('error');
  };

  return (
    <div className={styles.localFields}>
      <section aria-label={t('model_status')}>
        <strong>{selectedName}</strong>
        <p>
          {installed
            ? t('model_installed')
            : catalogError
              ? t('catalog_unavailable')
              : t('model_missing')}
        </p>
      </section>
      {(!Number.isInteger(lm.contextWindow) ||
        lm.contextWindow < 512 ||
        lm.contextWindow > 65536) && <p role="alert">{t('context_window_invalid')}</p>}
      <h3 className={styles.localHeading}>{t('local_runtime_section')}</h3>
      <div className={styles.localRuntimeRow}>
        <Button
          variant="primary"
          disabled={
            isStreaming ||
            !installed ||
            Boolean(catalogError) ||
            catalogLoading ||
            !Number.isInteger(lm.contextWindow) ||
            lm.contextWindow < 512 ||
            lm.contextWindow > 65536 ||
            startStatus === 'pending' ||
            lm.runtime.llm.state === 'starting'
          }
          data-status={startStatus}
          onClick={() => {
            void handleStart();
          }}
        >
          <Icons.Sparkle size={16} />
          {startStatus === 'pending'
            ? tLocal('runtime_starting')
            : lm.runtime.llm.state === 'failed' || startStatus === 'error'
              ? t('repair_model')
              : t('start_model')}
        </Button>
        {startStatus === 'error' && (
          <span role="alert" className={styles.localErrorChip}>
            {tLocal('runtime_start_error')}
          </span>
        )}
        <Button
          disabled={isStreaming || stopStatus === 'pending' || startStatus === 'pending'}
          data-status={stopStatus}
          onClick={() => {
            void handleStop();
          }}
        >
          <Icons.Stop size={15} />
          {stopStatus === 'pending' ? tLocal('runtime_stopping') : tLocal('stop_runtimes')}
        </Button>
        {stopStatus === 'error' && (
          <span role="alert" className={styles.localErrorChip}>
            {tLocal('runtime_stop_error')}
          </span>
        )}
        {lm.runtimeStatusKnown &&
        lm.runtime.llm.state === 'ready' &&
        !selectedLocalModelIsRunning(lm) ? (
          <span role="status">{t('different_model_running')}</span>
        ) : lm.runtimeStatusKnown ? (
          <RuntimeStatusPill label={tLocal('runtime_pill_llm')} state={lm.runtime.llm} />
        ) : (
          <span role="status">{t('runtime_unknown')}</span>
        )}
        <Button disabled={checking} onClick={() => void checkStatus()}>
          {checking ? t('checking') : t('check_status')}
        </Button>
      </div>

      <LocalLlmModelSelector
        activeModelIds={Object.keys(WIRE_ID_TO_LOCAL_MODEL)}
        activeId={activeWireId}
        onActiveChange={onActiveLocalChange}
        agentTurnInFlight={isStreaming || startStatus === 'pending' || stopStatus === 'pending'}
      />

      <CollapsibleCard title={t('advanced')}>
        <Field label={t('context_window')} helper={t('context_restart_hint')}>
          {(props) => (
            <input
              {...props}
              type="number"
              min={512}
              max={65536}
              step={512}
              value={lm.contextWindow}
              onChange={(event) => lm.setContextWindow(Number(event.target.value))}
              disabled={isStreaming || startStatus === 'pending'}
            />
          )}
        </Field>
        <h3 className={styles.localHeading}>{tLocal('image_model')}</h3>
        <LocalModelCard
          entry={{ id: 'sdxl_turbo', name: 'SDXL-Turbo (fp16)', size: 7e9 }}
          isLlm={false}
        />

        <Field label={tLocal('vram_strategy')}>
          {({ id }) => (
            <Select
              id={id}
              value={lm.vramStrategy}
              onChange={(value) => lm.setVramStrategy(value as VramStrategy)}
              ariaLabel={tLocal('vram_strategy')}
              options={[
                { value: 'auto-swap', label: tLocal('strategy_auto_swap') },
                { value: 'keep-both-loaded', label: tLocal('strategy_keep_both') },
                { value: 'disable-image-gen', label: tLocal('strategy_disable_image') },
              ]}
            />
          )}
        </Field>
      </CollapsibleCard>
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
