/**
 * SavesScreen (M5 P2.13).
 *
 * Two-page tome modal that lists the saves for the active session and shows
 * detail for a selected save on the right page. v1 is linear-only; the
 * Branches tab and BranchTree visualisation arrive in v2.
 */

import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { SaveSummary, SaveTag } from '../api/saves';
import saveThumbCombat from '../assets/save-thumb-combat.png';
import saveThumbDialog from '../assets/save-thumb-dialog.png';
import saveThumbExploration from '../assets/save-thumb-exploration.png';
import saveThumbNpc from '../assets/save-thumb-npc.png';
import { useSaves } from '../hooks/useSaves';
import { Icons } from '../ui/Icons';

const TAG_THUMB_SRC: Record<SaveTag, string> = {
  combat: saveThumbCombat,
  exploration: saveThumbExploration,
  dialog: saveThumbDialog,
  npc: saveThumbNpc,
};

type Tab = 'all' | 'manual' | 'auto';

const TAG_ORDER: SaveTag[] = ['combat', 'exploration', 'dialog', 'npc'];

function tagIcon(tag: SaveTag) {
  switch (tag) {
    case 'combat':
      return Icons.Sword;
    case 'exploration':
      return Icons.Compass;
    case 'dialog':
      return Icons.Scroll;
    case 'npc':
      return Icons.User;
  }
}

function kindIcon(kind: string) {
  switch (kind) {
    case 'manual':
      return Icons.Save;
    case 'checkpoint':
      return Icons.Book;
    default:
      return Icons.Sparkle;
  }
}

interface SaveThumbProps {
  tag: SaveTag;
  large?: boolean;
}

function SaveThumb({ tag, large = false }: SaveThumbProps) {
  return (
    <div className={`dm-save-thumb${large ? ' dm-save-thumb-lg' : ''}`} data-tag={tag}>
      <img src={TAG_THUMB_SRC[tag]} alt="" className="dm-save-thumb-art" />
    </div>
  );
}

interface SaveRowProps {
  save: SaveSummary;
  active: boolean;
  onClick: () => void;
  tagLabel: string;
  kindLabel: string;
  untitledLabel: string;
}

function SaveRow({ save, active, onClick, tagLabel, kindLabel, untitledLabel }: SaveRowProps) {
  const KindIcon = kindIcon(save.kind);
  const TagIcon = tagIcon(save.tag);
  return (
    <button
      type="button"
      className={`dm-save-row${active ? ' is-active' : ''}`}
      onClick={onClick}
      data-save-id={save.id}
    >
      <SaveThumb tag={save.tag} />
      <div className="dm-save-row-body">
        <div className="dm-save-row-title">{save.title || untitledLabel}</div>
        {save.summary && <div className="dm-save-row-summary">{save.summary}</div>}
        <div className="dm-save-row-meta">
          <span className="dm-tag" data-tag={save.tag}>
            <TagIcon size={10} /> {tagLabel}
          </span>
          <span className="dm-tag dm-tag-ghost">
            <KindIcon size={10} /> {kindLabel}
          </span>
          <span className="dm-save-row-date">
            {new Date(save.created_at).toLocaleString(undefined, {
              month: 'short',
              day: '2-digit',
              hour: '2-digit',
              minute: '2-digit',
            })}
          </span>
        </div>
      </div>
    </button>
  );
}

export function SavesScreen() {
  const { t } = useTranslation('saves');
  const { saves, selectedSaveId, refresh, deleteSave, loadSave, manualSave, selectSave, close } =
    useSaves();
  const [tab, setTab] = useState<Tab>('all');
  const [search, setSearch] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);

  // Refresh on mount so the modal always shows the canonical list.
  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Focus the container so Escape can close on the very first keystroke.
  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  const filtered = useMemo(() => {
    return saves.filter((s) => {
      if (tab === 'manual' && s.kind !== 'manual' && s.kind !== 'checkpoint') return false;
      if (tab === 'auto' && s.kind !== 'auto') return false;
      if (search) {
        const haystack = `${s.title} ${s.summary}`.toLowerCase();
        if (!haystack.includes(search.toLowerCase())) return false;
      }
      return true;
    });
  }, [saves, tab, search]);

  const selected = useMemo(
    () => filtered.find((s) => s.id === selectedSaveId) ?? null,
    [filtered, selectedSaveId],
  );

  const tagLabel = (tag: SaveTag) => t(`tag_${tag}`);
  const kindLabel = (kind: string) => {
    if (kind === 'manual' || kind === 'auto' || kind === 'checkpoint') return t(`kind_${kind}`);
    return kind;
  };

  const onLoad = async () => {
    if (!selected) return;
    await loadSave(selected.id);
    // v1: load is a no-op on game state (linear save model). The UI
    // simply confirms the save exists; v2 will rehydrate chat/combat.
  };

  const onOverwrite = async () => {
    if (!selected) return;
    await manualSave({
      kind: 'manual',
      title: selected.title,
      summary: selected.summary,
      tag: selected.tag,
    });
  };

  const onDelete = async () => {
    if (!selected) return;
    // eslint-disable-next-line no-alert
    if (typeof window !== 'undefined' && !window.confirm(t('confirm_delete'))) return;
    await deleteSave(selected.id);
  };

  const onCreateNew = async () => {
    await manualSave({
      kind: 'manual',
      title: t('default_manual_title'),
      summary: '',
      tag: 'exploration',
    });
  };

  // Single chapter "All saves" group for v1 (per-row chapter metadata
  // belongs to the v2 schema).
  const groups = useMemo(() => {
    if (filtered.length === 0) return [] as Array<{ chapter: string; items: SaveSummary[] }>;
    const ordered = [...filtered].sort((a, b) => {
      // Ensure stable deterministic ordering when timestamps tie:
      // primary newest-first, secondary by tag order, tertiary by id.
      if (a.created_at !== b.created_at) return a.created_at < b.created_at ? 1 : -1;
      const ta = TAG_ORDER.indexOf(a.tag);
      const tb = TAG_ORDER.indexOf(b.tag);
      if (ta !== tb) return ta - tb;
      return a.id.localeCompare(b.id);
    });
    return [{ chapter: t('all_saves_chapter'), items: ordered }];
  }, [filtered, t]);

  return (
    <div
      className="dm-saves-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={t('title')}
      onClick={close}
      onKeyDown={(e) => {
        if (e.key === 'Escape') close();
      }}
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: stopPropagation only - keyboard a11y is on the parent overlay */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: inner tome only suppresses click bubbling; the overlay above owns Escape */}
      <div
        className="dm-saves-tome"
        onClick={(e) => e.stopPropagation()}
        ref={containerRef}
        tabIndex={-1}
      >
        <button
          type="button"
          className="dm-saves-close"
          onClick={close}
          aria-label={t('close_aria')}
        >
          <Icons.X size={14} />
        </button>

        {/* Left page: list */}
        <div className="dm-saves-page dm-saves-page-left">
          <div className="dm-saves-header">
            <h1 className="dm-saves-title">{t('title')}</h1>
            <div className="dm-saves-subtitle">{t('subtitle')}</div>
          </div>

          <div className="dm-saves-toolbar">
            <div className="dm-saves-tabs" role="tablist">
              {(['all', 'manual', 'auto'] as Tab[]).map((k) => (
                <button
                  key={k}
                  type="button"
                  role="tab"
                  aria-selected={tab === k}
                  className={`dm-saves-tab${tab === k ? ' is-active' : ''}`}
                  onClick={() => setTab(k)}
                >
                  {t(`tab_${k}`)}
                </button>
              ))}
            </div>
            <div className="dm-saves-search">
              <input
                type="search"
                placeholder={t('search_placeholder')}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                aria-label={t('search_placeholder')}
              />
            </div>
          </div>

          <div className="dm-saves-list">
            {groups.length === 0 ? (
              <div className="dm-saves-empty">{t('empty_list')}</div>
            ) : (
              groups.map(({ chapter, items }) => (
                <div key={chapter} className="dm-saves-chapter">
                  <div className="dm-saves-chapter-header">
                    <span className="dm-saves-chapter-title">{chapter}</span>
                    <span className="dm-saves-chapter-count">{items.length}</span>
                  </div>
                  {items.map((s) => (
                    <SaveRow
                      key={s.id}
                      save={s}
                      active={s.id === selectedSaveId}
                      onClick={() => selectSave(s.id)}
                      tagLabel={tagLabel(s.tag)}
                      kindLabel={kindLabel(s.kind)}
                      untitledLabel={t('untitled')}
                    />
                  ))}
                </div>
              ))
            )}
          </div>

          <div className="dm-saves-actions-bar">
            <button
              type="button"
              className="dm-btn dm-btn-primary"
              onClick={() => void onCreateNew()}
            >
              <Icons.Plus size={14} /> {t('new_save')}
            </button>
            <span className="dm-saves-hint">{t('quick_save_hint')}</span>
          </div>
        </div>

        {/* Spine */}
        <div className="dm-saves-spine">
          <div className="dm-saves-spine-cord" />
          <div className="dm-saves-spine-cord" />
          <div className="dm-saves-spine-cord" />
        </div>

        {/* Right page: detail */}
        <div className="dm-saves-page">
          {selected === null ? (
            <div className="dm-saves-empty">{t('empty')}</div>
          ) : (
            <div className="dm-save-detail">
              <div className="dm-save-detail-illustration">
                <SaveThumb tag={selected.tag} large />
              </div>

              <div className="dm-save-detail-tags">
                <span className="dm-tag" data-tag={selected.tag}>
                  {tagLabel(selected.tag)}
                </span>
                <span className="dm-tag dm-tag-ghost">{kindLabel(selected.kind)}</span>
              </div>

              <h2 className="dm-save-detail-title">{selected.title || t('untitled')}</h2>

              <div className="dm-save-detail-summary">
                <div className="dm-save-detail-section-title">{t('summary_header')}</div>
                <p>{selected.summary || '-'}</p>
              </div>

              <div className="dm-save-detail-meta">
                <div className="dm-save-detail-meta-cell">
                  <div className="dm-save-detail-meta-label">{t('created_label')}</div>
                  <div className="dm-save-detail-meta-value">
                    {new Date(selected.created_at).toLocaleString()}
                  </div>
                </div>
              </div>

              <div className="dm-save-detail-actions">
                <button type="button" className="dm-btn-tb" onClick={() => void onLoad()}>
                  <Icons.ArrowReverse size={14} /> {t('load')}
                </button>
                <button type="button" className="dm-btn-tb" onClick={() => void onOverwrite()}>
                  <Icons.Save size={14} /> {t('overwrite')}
                </button>
                <button type="button" className="dm-btn-tb" onClick={() => void onDelete()}>
                  <Icons.X size={14} /> {t('delete')}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
