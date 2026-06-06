import { openUrl } from '@tauri-apps/plugin-opener';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { addManifest, type HfModel, type SearchParams } from '../../../api/hf';
import { useHfSearchStore } from '../../../state/hfSearch';
import { HfResultCard } from './HfResultCard';
import styles from './HfSearch.module.css';
import { HfSearchFilters } from './HfSearchFilters';
import { HfTokenRow } from './HfTokenRow';

const ARCH_PREFIXES = ['qwen', 'llama', 'mistral', 'phi', 'gemma'];
const LICENSE_TAG_PREFIX = 'license:';

/**
 * Derive a coarse arch label from HF tags. Mirrors `HfResultCard.detectArch`
 * but is permissive (prefix match) since we only need a string for the
 * manifest row, not the strict allowlist check.
 */
function deriveArch(tags: string[]): string {
  for (const raw of tags) {
    const tag = raw.toLowerCase();
    if (ARCH_PREFIXES.some((p) => tag.startsWith(p))) return tag;
  }
  return 'unknown';
}

/**
 * Map a sibling filename suffix to a quant label the backend manifest
 * understands. Mirrors `SUPPORTED_QUANT_SUFFIXES` in HfResultCard.
 */
function deriveQuant(filename: string): string {
  const lower = filename.toLowerCase();
  if (lower.endsWith('-q4_k_m.gguf')) return 'gguf-q4_k_m';
  if (lower.endsWith('-q5_k_m.gguf')) return 'gguf-q5_k_m';
  if (lower.endsWith('-q8_0.gguf')) return 'gguf-q8_0';
  if (lower.endsWith('-f16.gguf')) return 'gguf-f16';
  if (lower.endsWith('.safetensors')) return 'safetensors';
  return 'unknown';
}

function deriveLicense(tags: string[]): string {
  for (const tag of tags) {
    if (tag.startsWith(LICENSE_TAG_PREFIX)) return tag.slice(LICENSE_TAG_PREFIX.length);
  }
  return 'unknown';
}

function siblingSizeGb(model: HfModel, filename: string): number {
  const sib = model.siblings.find((s) => s.filename === filename);
  return (sib?.size ?? 0) / 1_000_000_000;
}

/**
 * Main "Search Hugging Face" panel: token row on top, then a `role="search"`
 * form, the filter row, an optional error line, and the result card list.
 * State lives in `useHfSearchStore` (M9-DM Task 19). The window `focus`
 * listener re-polls gated cards so users who just accepted a license on the
 * HF site see the badge clear when they tab back; the slice currently
 * implements that as a no-op pending M10.
 */
export function HfSearch() {
  const { t } = useTranslation('local_llm');
  const params = useHfSearchStore((s) => s.params);
  const results = useHfSearchStore((s) => s.results);
  const loading = useHfSearchStore((s) => s.loading);
  const error = useHfSearchStore((s) => s.error);
  const setQuery = useHfSearchStore((s) => s.setQuery);
  const setParam = useHfSearchStore((s) => s.setParam);
  const runSearch = useHfSearchStore((s) => s.runSearch);
  const repoll = useHfSearchStore((s) => s.repollGatedCards);
  const [addError, setAddError] = useState<string | null>(null);

  useEffect(() => {
    const handler = () => {
      void repoll();
    };
    window.addEventListener('focus', handler);
    return () => window.removeEventListener('focus', handler);
  }, [repoll]);

  return (
    <div className={styles.root}>
      <HfTokenRow />
      <search>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void runSearch();
          }}
          className={styles.searchBar}
        >
          <input
            type="search"
            value={params.q}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('search_placeholder')}
          />
          <button type="submit" disabled={loading}>
            {loading ? t('searching') : t('search')}
          </button>
        </form>
      </search>
      <HfSearchFilters
        value={params}
        onChange={(next) => {
          (Object.keys(next) as (keyof SearchParams)[]).forEach((k) => {
            const v = next[k];
            if (v !== undefined) setParam(k, v);
          });
        }}
      />
      {error && <p className={styles.error}>{error}</p>}
      {addError && (
        <p className={styles.error} role="alert">
          {t('hf_add_failed', { error: addError })}
        </p>
      )}
      <div>
        {results.map((m) => (
          <HfResultCard
            key={m.repo_id}
            model={m}
            onDownload={(model, filename, force) => {
              setAddError(null);
              void addManifest({
                repo_id: model.repo_id,
                hf_filename: filename,
                arch: deriveArch(model.tags),
                quant: deriveQuant(filename),
                size_gb: siblingSizeGb(model, filename),
                license: deriveLicense(model.tags),
                display_name: model.repo_id,
                force,
              }).catch((e: unknown) => {
                setAddError(e instanceof Error ? e.message : String(e));
              });
            }}
            onOpenHf={(repoId) => {
              void (async () => {
                try {
                  await openUrl(`https://huggingface.co/${repoId}`);
                } catch {
                  window.open(`https://huggingface.co/${repoId}`, '_blank');
                }
              })();
            }}
          />
        ))}
      </div>
    </div>
  );
}
