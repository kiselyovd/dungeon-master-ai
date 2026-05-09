import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocalRuntimeStatus } from '../hooks/useLocalRuntimeStatus';
import { useModelDownload } from '../hooks/useModelDownload';
import type { ModelId, VramStrategy } from '../state/localMode';
import { useStore } from '../state/useStore';
import styles from './LocalModeModal.module.css';
import { ModelDownloadCard } from './ModelDownloadCard';
import { RuntimeStatusPill } from './RuntimeStatusPill';

interface Props {
  open: boolean;
  onClose: () => void;
}

interface LlmEntry {
  id: ModelId;
  name: string;
  size: number;
  vram: number;
  warn?: string;
}

const LLMS: readonly LlmEntry[] = [
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

interface CardEntry {
  id: ModelId;
  name: string;
  size: number;
  vram?: number;
  warn?: string;
}

const ModelCard = ({ entry, isLlm }: { entry: CardEntry; isLlm: boolean }) => {
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
};

const persistConfig = (selectedLlm: ModelId, vramStrategy: VramStrategy) => {
  void fetch('/local-mode/config', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ selected_llm: selectedLlm, vram_strategy: vramStrategy }),
  }).catch(() => {
    // Backend may be down; the next save retry will surface the error.
  });
};

type RuntimeActionStatus = 'idle' | 'pending' | 'error';

const RESET_DELAY_MS = 3500;

export function LocalModeModal({ open, onClose }: Props) {
  const { t } = useTranslation('local_mode');
  const lm = useStore((s) => s.localMode);
  useLocalRuntimeStatus(open && lm.enabled);

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

  // When the modal closes, cancel any pending reset timers and clear status
  // so the next open does not flash a stale 'error' chip. The component stays
  // mounted (App.tsx always renders it), so the unmount cleanup above is not
  // enough on its own.
  useEffect(() => {
    if (!open) {
      clearStartReset();
      clearStopReset();
      setStartStatus('idle');
      setStopStatus('idle');
    }
  }, [open, clearStartReset, clearStopReset]);

  const handleStart = useCallback(async () => {
    clearStartReset();
    setStartStatus('pending');
    try {
      const res = await fetch('/local/runtime/start', { method: 'POST' });
      if (!res.ok) throw new Error(`http ${res.status}`);
      setStartStatus('idle');
    } catch {
      setStartStatus('error');
      startResetRef.current = setTimeout(() => {
        setStartStatus('idle');
        startResetRef.current = null;
      }, RESET_DELAY_MS);
    }
  }, [clearStartReset]);

  const handleStop = useCallback(async () => {
    clearStopReset();
    setStopStatus('pending');
    try {
      const res = await fetch('/local/runtime/stop', { method: 'POST' });
      if (!res.ok) throw new Error(`http ${res.status}`);
      setStopStatus('idle');
    } catch {
      setStopStatus('error');
      stopResetRef.current = setTimeout(() => {
        setStopStatus('idle');
        stopResetRef.current = null;
      }, RESET_DELAY_MS);
    }
  }, [clearStopReset]);

  if (!open) return null;

  return (
    <div
      className={styles.backdrop}
      role="dialog"
      aria-modal="true"
      aria-labelledby="local-mode-title"
    >
      <div className={styles.modal}>
        <h2 id="local-mode-title">{t('title')}</h2>

        <div className={styles.row}>
          <label>
            <input
              type="checkbox"
              checked={lm.enabled}
              onChange={(e) => {
                lm.setEnabled(e.target.checked);
                persistConfig(lm.selectedLlm, lm.vramStrategy);
              }}
            />
            {t('enable')}
          </label>
          <button
            type="button"
            disabled={startStatus === 'pending'}
            data-status={startStatus}
            onClick={() => {
              void handleStart();
            }}
          >
            {startStatus === 'pending' ? t('runtime_starting') : t('start_runtimes')}
          </button>
          {startStatus === 'error' && (
            <span role="alert" className={styles.errorChip}>
              {t('runtime_start_error')}
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
            {stopStatus === 'pending' ? t('runtime_stopping') : t('stop_runtimes')}
          </button>
          {stopStatus === 'error' && (
            <span role="alert" className={styles.errorChip}>
              {t('runtime_stop_error')}
            </span>
          )}
          <RuntimeStatusPill label="LLM" state={lm.runtime.llm} />
          <RuntimeStatusPill label="Image" state={lm.runtime.image} />
        </div>

        <h3>{t('llm_models')}</h3>
        {LLMS.map((m) => (
          <ModelCard key={m.id} entry={m} isLlm />
        ))}

        <h3>{t('image_model')}</h3>
        <ModelCard
          entry={{ id: 'sdxl_turbo', name: 'SDXL-Turbo (fp16)', size: 7e9 }}
          isLlm={false}
        />

        <h3>{t('vram_strategy')}</h3>
        <select
          className={styles.strategySelect}
          value={lm.vramStrategy}
          onChange={(e) => {
            const next = e.target.value as VramStrategy;
            lm.setVramStrategy(next);
            persistConfig(lm.selectedLlm, next);
          }}
        >
          <option value="auto-swap">{t('strategy_auto_swap')}</option>
          <option value="keep-both-loaded">{t('strategy_keep_both')}</option>
          <option value="disable-image-gen">{t('strategy_disable_image')}</option>
        </select>

        <div className={styles.closeRow}>
          <button type="button" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
