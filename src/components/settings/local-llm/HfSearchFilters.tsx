import { useTranslation } from 'react-i18next';
import type { SearchParams } from '../../../api/hf';
import styles from './HfSearchFilters.module.css';

export interface HfSearchFiltersProps {
  value: SearchParams;
  onChange: (next: SearchParams) => void;
}

/**
 * Strip a single key off `SearchParams` when its select is cleared back to
 * the "any" option. Project uses `exactOptionalPropertyTypes: true`, so we
 * cannot assign `undefined` to an optional field; the key must actually be
 * absent from the resulting object.
 */
function withoutKey<K extends keyof SearchParams>(obj: SearchParams, key: K): SearchParams {
  const { [key]: _omit, ...rest } = obj;
  return rest as SearchParams;
}

/**
 * Four-select filter row above the Hugging Face search results. Drives
 * `SearchParams` directly so the parent can pass the resulting object
 * straight to `api/hf.search`. Empty string in any select clears that
 * filter (the key is removed entirely to satisfy
 * `exactOptionalPropertyTypes`).
 */
export function HfSearchFilters({ value, onChange }: HfSearchFiltersProps) {
  const { t } = useTranslation('local_llm');
  return (
    <div className={styles.filters}>
      <select
        value={value.arch ?? ''}
        onChange={(e) => {
          const v = e.target.value;
          onChange(v ? { ...value, arch: v } : withoutKey(value, 'arch'));
        }}
        aria-label={t('filter_arch')}
      >
        <option value="">{t('filter_any_arch')}</option>
        <option value="qwen3">Qwen3</option>
        <option value="qwen2">Qwen2</option>
        <option value="llama3">Llama 3</option>
        <option value="mistral">Mistral</option>
        <option value="phi3">Phi-3</option>
        <option value="gemma2">Gemma 2</option>
      </select>
      <select
        value={value.size ?? ''}
        onChange={(e) => {
          const v = e.target.value as '' | NonNullable<SearchParams['size']>;
          onChange(v ? { ...value, size: v } : withoutKey(value, 'size'));
        }}
        aria-label={t('filter_size')}
      >
        <option value="">{t('filter_any_size')}</option>
        <option value="small">&lt; 4 GB</option>
        <option value="medium">4-8 GB</option>
        <option value="large">&gt; 8 GB</option>
      </select>
      <select
        value={value.license ?? ''}
        onChange={(e) => {
          const v = e.target.value;
          onChange(v ? { ...value, license: v } : withoutKey(value, 'license'));
        }}
        aria-label={t('filter_license')}
      >
        <option value="">{t('filter_any_license')}</option>
        <option value="apache-2.0">Apache 2.0</option>
        <option value="mit">MIT</option>
        <option value="llama3">Llama 3</option>
      </select>
      <select
        value={value.sort ?? 'downloads'}
        onChange={(e) =>
          onChange({ ...value, sort: e.target.value as NonNullable<SearchParams['sort']> })
        }
        aria-label={t('filter_sort')}
      >
        <option value="downloads">{t('sort_downloads')}</option>
        <option value="likes">{t('sort_likes')}</option>
        <option value="last-modified">{t('sort_recent')}</option>
      </select>
    </div>
  );
}
