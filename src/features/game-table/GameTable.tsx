import { type ReactNode, useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { resetDesktopGeometry } from '../../api/desktop';
import { KEY_ART, KEY_ART_IDS } from '../../assets/livingTabletop';
import { useStore } from '../../state/useStore';
import {
  CONTEXT_PANELS,
  MAX_CONTEXT_WIDTH,
  MIN_CONTEXT_WIDTH,
} from '../../state/workspacePreferences';
import { Button } from '../../ui/Button';
import { DecorativeImage } from '../../ui/DecorativeImage';
import { Icons } from '../../ui/Icons';
import { Modal } from '../../ui/Modal';
import { PaneResizer } from '../../ui/PaneResizer';
import styles from './GameTable.module.css';
import type { TableActor } from './model/types';
import { PartyStrip } from './PartyStrip';
import { useTablePresentation } from './TableContext';

export interface GameTableSlots {
  commandLauncher?: ReactNode;
  contextPanels?: readonly { id: string; label: string; content: ReactNode }[] | undefined;
  activeContext?: string | null | undefined;
  onContextChange?: ((id: string | null) => void) | undefined;
  scene: ReactNode;
  chat: ReactNode;
  notices?: ReactNode;
  context?: ReactNode;
  roomControls?: ReactNode;
  resizer?: ReactNode;
  renderPortrait?: ((actor: TableActor) => ReactNode) | undefined;
}
/** One bounded viewport. Scene and conversation own their scrolling; the
 * conversation adapter owns the composer and its pending identity/draft. */
export function GameTable({
  commandLauncher,
  contextPanels,
  activeContext,
  onContextChange,
  scene,
  chat,
  notices,
  context,
  roomControls,
  resizer,
  renderPortrait,
}: GameTableSlots) {
  const { model, ports } = useTablePresentation();
  const { t } = useTranslation('rooms');
  const { t: tc } = useTranslation('common');
  const [panel, setPanel] = useState<'room' | null>(null);
  const workspace = useStore((state) => state.settings.workspace);
  const setWorkspace = useStore((state) => state.settings.setWorkspace);
  const resetWorkspace = useStore((state) => state.settings.resetWorkspace);
  const [resetFailure, setResetFailure] = useState(false);
  const resetLayout = async () => {
    resetWorkspace();
    setResetFailure(false);
    try {
      await resetDesktopGeometry();
    } catch {
      setResetFailure(true);
      console.warn('[workspace.reset]', { code: 'desktop_reset_failed' });
    }
  };
  const expandedContext = workspace.expandedContext;
  const setExpandedContext = (expandedContext: boolean) => setWorkspace({ expandedContext });
  const panels =
    contextPanels ??
    (context ? [{ id: 'context', label: t('table_context'), content: context }] : []);
  const selected = workspace.selectedContext;
  const lastControlled = useRef(activeContext);
  const notifyContext = useRef(onContextChange);
  notifyContext.current = onContextChange;
  const controlled = useRef(activeContext);
  controlled.current = activeContext;
  useEffect(() => {
    if (activeContext === lastControlled.current) return;
    lastControlled.current = activeContext;
    const selectedContext = CONTEXT_PANELS.find((id) => id === activeContext) ?? null;
    if (selectedContext !== useStore.getState().settings.workspace.selectedContext)
      setWorkspace({ selectedContext });
  }, [activeContext, setWorkspace]);
  useEffect(() => {
    if (controlled.current !== workspace.selectedContext)
      notifyContext.current?.(workspace.selectedContext);
  }, [workspace.selectedContext]);
  useEffect(() => {
    document.documentElement.style.setProperty('--context-width', `${workspace.contextWidth}px`);
  }, [workspace.contextWidth]);
  const chosen = panels.find((item) => item.id === selected);
  const selectContext = (id: string | null) => {
    setWorkspace({ selectedContext: CONTEXT_PANELS.find((panel) => panel === id) ?? null });
    onContextChange?.(id);
    if (id === null) setExpandedContext(false);
  };
  const opener = useRef<HTMLElement | null>(null);
  const wasOpen = useRef(false);
  const panelId = useId();
  useEffect(() => {
    if (!selected) return;
    const close = (event: KeyboardEvent) => {
      if (
        event.key !== 'Escape' ||
        event.defaultPrevented ||
        document.querySelector('[role="dialog"]')
      )
        return;
      event.preventDefault();
      setWorkspace({ selectedContext: null, expandedContext: false });
      onContextChange?.(null);
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [selected, onContextChange, setWorkspace]);
  useEffect(() => {
    if (chosen && !wasOpen.current) opener.current = document.activeElement as HTMLElement;
    if (!chosen && wasOpen.current) opener.current?.focus();
    wasOpen.current = Boolean(chosen);
  }, [chosen]);
  const closeContext = () => selectContext(null);
  const contextBody = chosen && (
    <>
      <div role="tablist" aria-label={t('table_context')} className={styles.tabs}>
        {panels.map((item) => (
          <Button
            key={item.id}
            role="tab"
            aria-selected={chosen.id === item.id}
            aria-controls={panelId}
            tabIndex={chosen.id === item.id ? 0 : -1}
            onClick={() => selectContext(item.id)}
            onKeyDown={(event) => {
              if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
              event.preventDefault();
              const next =
                panels[
                  (panels.indexOf(item) + (event.key === 'ArrowRight' ? 1 : -1) + panels.length) %
                    panels.length
                ];
              if (next) {
                selectContext(next.id);
                event.currentTarget.parentElement
                  ?.querySelectorAll<HTMLElement>('[role="tab"]')
                  [panels.indexOf(next)]?.focus();
              }
            }}
          >
            {item.label}
          </Button>
        ))}
      </div>
      <div id={panelId} role="tabpanel" aria-label={chosen.label} className={styles.contextContent}>
        {chosen.content}
      </div>
    </>
  );
  return (
    <section
      className={styles.root}
      data-table-layout="persistent"
      data-mode={model.scope.mode}
      data-view-mode={workspace.viewMode}
    >
      <div className={styles.top}>
        <PartyStrip actors={model.actors} renderPortrait={renderPortrait} />
        <div className={styles.controls}>
          {commandLauncher}
          <fieldset className={styles.workspaceViews} aria-label={t('workspace_views')}>
            {(['split', 'scene', 'chat'] as const).map((mode) => (
              <Button
                key={mode}
                aria-pressed={workspace.viewMode === mode}
                onClick={() => setWorkspace({ viewMode: mode })}
              >
                {t(`workspace_${mode}`)}
              </Button>
            ))}
          </fieldset>
          <Button
            onClick={() => void resetLayout()}
            aria-label={t('workspace_reset')}
            title={t('workspace_reset')}
          >
            <Icons.Refresh size={14} />
          </Button>
          {panels.length > 0 && (
            <Button
              aria-expanded={Boolean(chosen)}
              onClick={() => selectContext(chosen ? null : (panels[0]?.id ?? null))}
            >
              {t('table_context')}
            </Button>
          )}
          {ports.configure && model.scope.mode !== 'solo' && (
            <Button onClick={ports.configure} disabled={!model.availability.configure.enabled}>
              {tc('settings')}
            </Button>
          )}
          {roomControls && (
            <Button data-command-target="room" onClick={() => setPanel('room')}>
              {t('table_room')}
            </Button>
          )}
        </div>
      </div>
      <div className={styles.body}>
        <div className={styles.stage}>
          <div className={styles.notices}>
            {notices}
            {resetFailure && <p role="status">{t('workspace_reset_failed')}</p>}
          </div>
          <div className={styles.scene}>
            {scene ?? (
              <div className={styles.exploration}>
                <DecorativeImage src={KEY_ART.vttEmpty} assetId={KEY_ART_IDS.vttEmpty} />
                <span>{model.scene.title ?? model.title}</span>
              </div>
            )}
            {chosen && !expandedContext && (
              <aside
                className={styles.contextDock}
                aria-label={t('table_context')}
                onKeyDown={(event) => {
                  if (
                    event.key === 'Escape' &&
                    !event.defaultPrevented &&
                    !document.querySelector('[role="dialog"]')
                  ) {
                    event.stopPropagation();
                    closeContext();
                  }
                }}
              >
                <PaneResizer
                  label={t('workspace_context_resize')}
                  value={workspace.contextWidth}
                  min={MIN_CONTEXT_WIDTH}
                  max={MAX_CONTEXT_WIDTH}
                  onCommit={(contextWidth) => setWorkspace({ contextWidth })}
                  onPreview={(width) =>
                    document.documentElement.style.setProperty('--context-width', `${width}px`)
                  }
                />
                <div className={styles.contextHeader}>
                  <strong>{chosen.label}</strong>
                  <Button aria-label={t('context_expand')} onClick={() => setExpandedContext(true)}>
                    <Icons.Square size={14} />
                  </Button>
                  <Button aria-label={tc('close')} onClick={closeContext}>
                    <Icons.X size={14} />
                  </Button>
                </div>
                {contextBody}
              </aside>
            )}
          </div>
        </div>
        <aside className={styles.chat} aria-label={t('dm')}>
          {resizer}
          {chat}
        </aside>
      </div>
      <Modal open={panel !== null} onClose={() => setPanel(null)} title={t('table_room')}>
        {roomControls}
      </Modal>
      <Modal
        open={Boolean(chosen && expandedContext)}
        title={chosen?.label ?? t('table_context')}
        onClose={closeContext}
      >
        {expandedContext && contextBody}
      </Modal>
    </section>
  );
}
