import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import { Modal } from '../../ui/Modal';
import styles from './CommandCenter.module.css';
import {
  type AppCommand,
  commandPlatform,
  filterCommands,
  matchesShortcut,
  PALETTE_SHORTCUT,
  shortcutLabel,
} from './model';

/** The app supplies role-aware callbacks; this view never invokes an API. */
export function CommandCenter({ commands }: { commands: readonly AppCommand[] }) {
  const { t } = useTranslation('common');
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [advanced, setAdvanced] = useState(false);
  const [failure, setFailure] = useState(false);
  const latest = useRef(commands);
  latest.current = commands;
  const pending = useRef(new Set<string>());
  const scheduled = useRef(new Set<number>());
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      for (const timer of scheduled.current) window.clearTimeout(timer);
      scheduled.current.clear();
    };
  }, []);
  const platform = commandPlatform(navigator.platform);
  const openPalette = () => {
    setQuery('');
    setFailure(false);
    setOpen(true);
  };
  const execute = (id: string) => {
    const command = latest.current.find((item) => item.id === id);
    if (!command?.enabled || pending.current.has(id)) {
      console.debug('[command]', { id, result: 'refused' });
      return;
    }
    pending.current.add(id);
    setOpen(false);
    // Let Modal restore its opener before a command focuses another view.
    const timer = window.setTimeout(() => {
      scheduled.current.delete(timer);
      void Promise.resolve()
        .then(() => {
          const current = latest.current.find((item) => item.id === id);
          if (current?.enabled) {
            console.debug('[command]', { id, result: 'invoked' });
            return current.run();
          }
          console.debug('[command]', { id, result: 'refused' });
        })
        .catch(() => {
          console.warn('[command]', { id, code: 'command_failed' });
          if (!mounted.current) return;
          setFailure(true);
          setOpen(true);
        })
        .finally(() => pending.current.delete(id));
    }, 0);
    scheduled.current.add(timer);
  };
  const executeRef = useRef(execute);
  executeRef.current = execute;
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (matchesShortcut(event, PALETTE_SHORTCUT, platform, true)) {
        event.preventDefault();
        if (event.repeat) return;
        if (!open) {
          setQuery('');
          setFailure(false);
          setOpen(true);
        }
        return;
      }
      const command = latest.current.find(
        (item) => item.shortcut && matchesShortcut(event, item.shortcut, platform, true),
      );
      if (!command) return;
      event.preventDefault();
      if (document.querySelector('[role="dialog"]') && command.id !== 'fullscreen') return;
      if (event.repeat) return;
      executeRef.current(command.id);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, platform]);
  const visible = filterCommands(commands, query, advanced);
  return (
    <>
      <button
        type="button"
        className="dm-btn-tb"
        onClick={openPalette}
        title={`${t('commands_open')} (${shortcutLabel(PALETTE_SHORTCUT, platform)})`}
        aria-keyshortcuts={platform === 'mac' ? 'Meta+K' : 'Control+K'}
      >
        {t('commands_open')}
      </button>
      <Modal
        open={open}
        onClose={() => setOpen(false)}
        title={t('commands_open')}
        footer={<Button onClick={() => setOpen(false)}>{t('close')}</Button>}
      >
        <Field label={t('commands_search')}>
          {(props) => (
            <input
              {...props}
              className={styles.search}
              value={query}
              autoComplete="off"
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) return;
                if (event.key === 'ArrowDown') {
                  event.preventDefault();
                  event.currentTarget
                    .closest('[role="dialog"]')
                    ?.querySelector<HTMLElement>('[data-command-id]:not([disabled])')
                    ?.focus();
                }
                if (
                  event.key === 'Enter' &&
                  !event.repeat &&
                  visible.length === 1 &&
                  visible[0]?.enabled
                ) {
                  event.preventDefault();
                  execute(visible[0].id);
                }
              }}
            />
          )}
        </Field>
        <label className={styles.advanced}>
          <input
            type="checkbox"
            checked={advanced}
            onChange={(event) => setAdvanced(event.target.checked)}
          />
          {t('commands_advanced')}
        </label>
        {failure && <p role="alert">{t('commands_failed')}</p>}
        <ul className={styles.list} aria-label={t('commands_results')}>
          {visible.map((command) => (
            <li key={command.id}>
              <Button
                className={styles.command}
                data-command-id={command.id}
                disabled={!command.enabled}
                onClick={() => execute(command.id)}
                title={!command.enabled ? command.unavailable : undefined}
              >
                <span>
                  {command.label}
                  {!command.enabled && (
                    <small className={styles.reason}>
                      {command.unavailable ?? t('commands_unavailable')}
                    </small>
                  )}
                </span>
                {command.shortcut && <kbd>{shortcutLabel(command.shortcut, platform)}</kbd>}
              </Button>
            </li>
          ))}
        </ul>
        {visible.length === 0 && <p role="status">{t('commands_empty')}</p>}
      </Modal>
    </>
  );
}
