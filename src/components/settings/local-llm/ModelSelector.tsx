import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import { useDownloadEvents } from '../../../hooks/useDownloadEvents';
import { mergeManifests } from '../../../state/local_llm/manifest';
import { useLocalLlmStore } from '../../../state/localLlm';
import { ActiveModelPicker } from './ActiveModelPicker';
import { CollapsibleCard } from './CollapsibleCard';
import { HfSearch } from './HfSearch';
import { ManageDownloads } from './ManageDownloads';
import styles from './ModelSelector.module.css';

export interface ModelSelectorProps {
  /** Currently-active local model id (matches a MergedEntry.id). */
  activeId: string | null;
  /** Notified when the user picks a different installed model. */
  onActiveChange: (id: string) => void;
  /**
   * If true, the picker fieldset is disabled so the user cannot swap models
   * mid-turn. Wired by the parent from `session.agentTurnInFlight`.
   */
  agentTurnInFlight: boolean;
}

/**
 * Three-section container for the local LLM picker (M9-DM Task 14):
 *
 *   1. Active model -> `ActiveModelPicker` (installed-only radio list)
 *   2. Manage downloads (collapsible) -> `ManageDownloads`
 *   3. Search Hugging Face (collapsible) -> `HfSearch` placeholder (Task 19)
 *
 * Loads the manifest on mount from `GET /local-llm/manifest`, merges system +
 * user entries with install state via `useLocalLlmStore.merged()`, and feeds
 * the result into all three children. The container itself is purely glue;
 * the per-section UI lives in the sibling components.
 */
export function ModelSelector({ activeId, onActiveChange, agentTurnInFlight }: ModelSelectorProps) {
  const { t } = useTranslation('local_llm');
  const loadManifest = useLocalLlmStore((s) => s.loadManifest);
  const startDownload = useLocalLlmStore((s) => s.startDownload);
  const deleteModel = useLocalLlmStore((s) => s.deleteModel);

  useDownloadEvents();
  // Re-derive `merged` whenever any of the four backing fields change. We
  // subscribe to each via its own selector so unrelated `loading` / `error`
  // updates do not force a re-render, and so the `useMemo` dependency array
  // can list the actual inputs the merge consumes (lets biome's exhaustive
  // hook-deps lint stay happy).
  const system = useLocalLlmStore((s) => s.system);
  const user = useLocalLlmStore((s) => s.user);
  const installedIds = useLocalLlmStore((s) => s.installedIds);
  const downloadStates = useLocalLlmStore((s) => s.downloadStates);
  const merged = useMemo(
    () => mergeManifests(system, user, installedIds, downloadStates),
    [system, user, installedIds, downloadStates],
  );

  const installedCount = useMemo(() => merged.filter((m) => m.installed).length, [merged]);
  const totalCount = merged.length;
  const installed = useMemo(() => merged.filter((m) => m.installed), [merged]);

  useEffect(() => {
    void loadManifest();
  }, [loadManifest]);

  return (
    <div className={styles.modelSelector}>
      <section>
        <h4 className={styles.sectionTitle}>{t('active_model_section')}</h4>
        <ActiveModelPicker
          installedModels={installed}
          activeId={activeId}
          onChange={onActiveChange}
          disabled={agentTurnInFlight}
        />
      </section>

      <CollapsibleCard
        title={t('manage_downloads')}
        chip={t('installed_count', { installed: installedCount, total: totalCount })}
      >
        <ManageDownloads
          models={merged}
          onDownload={(id) => void startDownload(id)}
          onDelete={(id) => void deleteModel(id)}
        />
      </CollapsibleCard>

      <CollapsibleCard title={t('search_hf')} chip="12k+">
        <HfSearch />
      </CollapsibleCard>
    </div>
  );
}
