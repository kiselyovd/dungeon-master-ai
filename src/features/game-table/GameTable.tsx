import { type ReactNode, useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { KEY_ART } from '../../assets/livingTabletop';
import { Button } from '../../ui/Button';
import { Icons } from '../../ui/Icons';
import { Modal } from '../../ui/Modal';
import styles from './GameTable.module.css';
import type { TableActor } from './model/types';
import { PartyStrip } from './PartyStrip';
import { useTablePresentation } from './TableContext';

export interface GameTableSlots {
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
  const [localContext, setLocalContext] = useState<string | null>(null);
  const [expandedContext, setExpandedContext] = useState(false);
  const panels =
    contextPanels ??
    (context ? [{ id: 'context', label: t('table_context'), content: context }] : []);
  const selected = onContextChange ? activeContext : localContext;
  const chosen = panels.find((item) => item.id === selected);
  const selectContext = (id: string | null) => {
    if (onContextChange) onContextChange(id);
    else setLocalContext(id);
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
      if (onContextChange) onContextChange(null);
      else setLocalContext(null);
      setExpandedContext(false);
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [selected, onContextChange]);
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
    <section className={styles.root} data-table-layout="persistent" data-mode={model.scope.mode}>
      <div className={styles.top}>
        <PartyStrip actors={model.actors} renderPortrait={renderPortrait} />
        <div className={styles.controls}>
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
          {roomControls && <Button onClick={() => setPanel('room')}>{t('table_room')}</Button>}
        </div>
      </div>
      <div className={styles.body}>
        <div className={styles.stage}>
          <div className={styles.notices}>{notices}</div>
          <div className={styles.scene}>
            {scene ?? (
              <div className={styles.exploration}>
                <img src={KEY_ART.vttEmpty} alt="" />
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
