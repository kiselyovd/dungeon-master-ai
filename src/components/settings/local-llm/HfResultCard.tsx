import { useTranslation } from 'react-i18next';
import type { HfModel } from '../../../api/hf';
import styles from './HfResultCard.module.css';

/**
 * Whitelist of model architectures the local-LLM runtime can actually load.
 * Mirrors the backend `SUPPORTED_ARCH` set in `src/hf/compat.rs` so the UI
 * and server agree on which Hugging Face results are "compatible".
 */
const SUPPORTED_ARCH = new Set([
  'qwen2',
  'qwen3',
  'llama2',
  'llama3',
  'llama',
  'mistral',
  'mixtral',
  'phi3',
  'gemma2',
  'gemma',
]);

/**
 * Quantisation / weight-file suffixes accepted by the loader. Mirrors the
 * backend `SUPPORTED_QUANT_SUFFIXES` list (see `src/hf/compat.rs`).
 */
const SUPPORTED_QUANT_SUFFIXES = [
  '-q4_k_m.gguf',
  '-q5_k_m.gguf',
  '-q8_0.gguf',
  '-f16.gguf',
  '.safetensors',
];

function detectArch(tags: string[]): string | null {
  for (const tag of tags.map((x) => x.toLowerCase())) {
    if (SUPPORTED_ARCH.has(tag)) return tag;
  }
  return null;
}

function pickSibling(model: HfModel): { filename: string; size_gb: number } | null {
  for (const s of model.siblings) {
    const lower = s.filename.toLowerCase();
    if (SUPPORTED_QUANT_SUFFIXES.some((sfx) => lower.endsWith(sfx))) {
      return { filename: s.filename, size_gb: (s.size ?? 0) / 1_000_000_000 };
    }
  }
  return null;
}

export interface HfResultCardProps {
  model: HfModel;
  onDownload: (model: HfModel, filename: string, force: boolean) => void;
  onOpenHf: (repoId: string) => void;
}

/**
 * Single Hugging Face search result. Renders one of three branches based on
 * model state:
 *   1. `gated` -> warning + "Open HF" deep-link (license acceptance flow)
 *   2. unsupported arch/quant -> warning + "Add anyway" (force=true)
 *   3. compatible -> standard "Download" button with arch/size/downloads meta
 *
 * Compatibility is determined locally via `SUPPORTED_ARCH` / `SUPPORTED_QUANT_SUFFIXES`,
 * which mirror the backend allowlist so the UI never offers a download the
 * server would reject.
 */
export function HfResultCard({ model, onDownload, onOpenHf }: HfResultCardProps) {
  const { t } = useTranslation('local_llm');
  const arch = detectArch(model.tags);
  const sibling = pickSibling(model);
  const compatible = arch !== null && sibling !== null;

  if (model.gated) {
    return (
      <div data-testid="hf-card-gated" className={`${styles.card} ${styles.cardGated}`}>
        <strong>{model.repo_id}</strong>
        <p className={styles.meta}>{t('hf_card_gated')}</p>
        <button type="button" onClick={() => onOpenHf(model.repo_id)}>
          {t('hf_open_hf')}
        </button>
      </div>
    );
  }

  if (!compatible) {
    return (
      <div data-testid="hf-card-unsupported" className={`${styles.card} ${styles.cardUnsupported}`}>
        <strong>{model.repo_id}</strong>
        <p className={styles.meta}>{t('hf_card_unsupported')}</p>
        <button
          type="button"
          onClick={() => {
            const sib = model.siblings[0];
            if (sib) onDownload(model, sib.filename, true);
          }}
        >
          {t('hf_add_anyway')}
        </button>
      </div>
    );
  }

  return (
    <div data-testid="hf-card-compatible" className={`${styles.card} ${styles.cardCompatible}`}>
      <strong>{model.repo_id}</strong>
      <p className={styles.meta}>
        {arch} | {sibling?.size_gb.toFixed(1)} GB | {model.downloads.toLocaleString()} dl
      </p>
      <button
        type="button"
        onClick={() => {
          if (sibling) onDownload(model, sibling.filename, false);
        }}
      >
        {t('download')}
      </button>
    </div>
  );
}
