/**
 * Presentation-only model picker. Renders the result of useDiscoverProvider
 * with three optional sections (Recommended for ModelSource=='curated',
 * Discovered for 'discovered-api'/'discovered-hf-hub') plus two disabled
 * placeholders (Custom HF repo / Search Hugging Face) that unlock in the
 * next M7.5-DM chunk.
 *
 * The free-text input stays on top and is always editable: per spec
 * anti-decision "no auto-correct of user-typed slugs" - a pasted custom slug
 * should pass straight through to the parent.
 */
import { type ReactNode, useId, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ResolvedModelEntry } from '../state/discoveredCatalogs';
import styles from './ModelSelector.module.css';

export interface ModelSelectorProps {
  value: string;
  onChange: (modelId: string) => void;
  models: ResolvedModelEntry[];
  status: 'idle' | 'loading' | 'error';
  error: string | null;
  onDiscover: () => void;
  lastCachedAt: string | null;
  placeholder?: string;
}

function formatTokens(n: number | undefined | null): string {
  if (!n) return '';
  if (n >= 1_000_000) return `${Math.round(n / 100_000) / 10}M`;
  if (n >= 1_000) return `${Math.round(n / 100) / 10}k`;
  return String(n);
}

function capabilityPills(m: ResolvedModelEntry, t: (k: string) => string): string[] {
  const pills: string[] = [];
  if (m.capabilities.vision_input) pills.push(t('cap_vision'));
  if (m.capabilities.reasoning) pills.push(t('cap_reasoning'));
  if (m.capabilities.tool_calls) pills.push(t('cap_tools'));
  const ctx = formatTokens(m.context_length);
  if (ctx) pills.push(ctx);
  return pills;
}

export function ModelSelector({
  value,
  onChange,
  models,
  status,
  error,
  onDiscover,
  lastCachedAt,
  placeholder,
}: ModelSelectorProps) {
  const { t } = useTranslation('settings');
  const [query, setQuery] = useState('');
  const textId = useId();
  const filterId = useId();

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return models;
    return models.filter(
      (m) => m.model_id.toLowerCase().includes(q) || m.display_name.toLowerCase().includes(q),
    );
  }, [models, query]);

  const recommended = filtered.filter((m) => m.source === 'curated');
  const discovered = filtered.filter(
    (m) => m.source === 'discovered-api' || m.source === 'discovered-hf-hub',
  );

  const isLoading = status === 'loading';
  const isError = status === 'error';
  const discoverLabel = isLoading ? t('discovering') : t('discover_models');

  return (
    <div className={styles.root}>
      <input
        id={textId}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder ?? t('model_selector_text_placeholder')}
        className={styles.freeText}
        aria-label={t('model_id_aria_label')}
      />

      <div className={styles.discoverRow}>
        <button
          type="button"
          onClick={onDiscover}
          disabled={isLoading}
          className={styles.discoverButton}
        >
          {discoverLabel}
        </button>
        {lastCachedAt ? (
          <span className={styles.cacheNote}>
            {t('last_cached_at', { time: new Date(lastCachedAt).toLocaleString() })}
          </span>
        ) : null}
      </div>

      {isError && error ? (
        <div role="alert" className={styles.errorBanner}>
          {error}
        </div>
      ) : null}

      {models.length > 0 ? (
        <input
          id={filterId}
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t('model_filter_placeholder')}
          className={styles.filterInput}
          aria-label={t('model_filter_aria_label')}
        />
      ) : null}

      {recommended.length > 0 ? (
        <Section label={t('section_recommended')} asListbox>
          {recommended.map((m) => (
            <ModelRow
              key={m.model_id}
              entry={m}
              selected={m.model_id === value}
              onSelect={onChange}
              pills={capabilityPills(m, t)}
            />
          ))}
        </Section>
      ) : null}

      {discovered.length > 0 ? (
        <Section label={t('section_discovered')} asListbox>
          {discovered.map((m) => (
            <ModelRow
              key={m.model_id}
              entry={m}
              selected={m.model_id === value}
              onSelect={onChange}
              pills={capabilityPills(m, t)}
            />
          ))}
        </Section>
      ) : null}

      <Section label={t('section_custom_hf')}>
        <p className={styles.placeholderHelper}>{t('model_selector_custom_disabled_helper')}</p>
      </Section>
      <Section label={t('section_search_hf')}>
        <p className={styles.placeholderHelper}>{t('model_selector_search_disabled_helper')}</p>
      </Section>
    </div>
  );
}

function Section({
  label,
  children,
  asListbox,
}: {
  label: string;
  children: ReactNode;
  asListbox?: boolean;
}) {
  return (
    <div className={styles.section}>
      <h4 className={styles.sectionLabel}>{label}</h4>
      <div
        className={styles.sectionBody}
        {...(asListbox ? { role: 'listbox', 'aria-label': label } : {})}
      >
        {children}
      </div>
    </div>
  );
}

function ModelRow({
  entry,
  selected,
  onSelect,
  pills,
}: {
  entry: ResolvedModelEntry;
  selected: boolean;
  onSelect: (modelId: string) => void;
  pills: string[];
}) {
  return (
    <button
      type="button"
      role="option"
      className={selected ? `${styles.row} ${styles.rowSelected}` : styles.row}
      onClick={() => onSelect(entry.model_id)}
      aria-selected={selected}
      data-selected={selected ? 'true' : 'false'}
    >
      <span className={styles.rowMain}>
        <span className={styles.rowDisplayName}>{entry.display_name}</span>
        {entry.display_name !== entry.model_id ? (
          <span className={styles.rowModelId}>{entry.model_id}</span>
        ) : null}
      </span>
      <span className={styles.rowPills}>
        {pills.map((p) => (
          <span key={p} className={styles.pill}>
            {p}
          </span>
        ))}
      </span>
    </button>
  );
}
