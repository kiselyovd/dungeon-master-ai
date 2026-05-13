/**
 * CharFab (M5 P2.14).
 *
 * Bottom-left FAB pill placed inside `.dm-vtt-panel` that opens the
 * CharacterSheet modal. Renders `<icon> <PC name> <hp/maxHp>`.
 *
 * The component itself does not own the modal open/close state - the
 * parent (App.tsx) wires the click handler. The fab is hidden when no
 * character has been created yet (`pc.name === null`).
 *
 * Right-click opens a small context menu with a "Create new character"
 * item that invokes the optional `onOpenWizard` callback.
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';

interface CharFabProps {
  onOpen: () => void;
  onOpenWizard?: () => void;
}

export function CharFab({ onOpen, onOpenWizard }: CharFabProps) {
  const { t } = useTranslation('character');
  const [menuOpen, setMenuOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const name = useStore((s) => s.pc.name);
  const hp = useStore((s) => s.pc.hp);
  const hpMax = useStore((s) => s.pc.hpMax);

  useEffect(() => {
    if (!menuOpen) return undefined;
    function onClickAway(e: MouseEvent) {
      if (!containerRef.current?.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    }
    document.addEventListener('mousedown', onClickAway);
    return () => document.removeEventListener('mousedown', onClickAway);
  }, [menuOpen]);

  if (name === null) return null;

  return (
    <div ref={containerRef} style={{ position: 'relative' }}>
      <button
        type="button"
        className="dm-char-fab"
        onClick={onOpen}
        onContextMenu={(e) => {
          if (onOpenWizard) {
            e.preventDefault();
            setMenuOpen(true);
          }
        }}
        aria-label={t('fab_aria', { name })}
      >
        <Icons.User size={14} />
        <span className="dm-char-fab-name">{name}</span>
        <span className="dm-char-fab-hp dm-mono">
          {hp}/{hpMax}
        </span>
      </button>
      {menuOpen && onOpenWizard && (
        <div
          role="menu"
          className="dm-char-fab-menu"
          style={{
            position: 'absolute',
            bottom: '100%',
            left: 0,
            marginBottom: 6,
            background: 'rgba(20,16,26,0.95)',
            border: '1px solid rgba(212,175,55,0.3)',
            padding: 4,
            borderRadius: 4,
            zIndex: 1000,
          }}
        >
          <button
            type="button"
            role="menuitem"
            className="dm-wizard-card"
            onClick={() => {
              setMenuOpen(false);
              onOpenWizard();
            }}
          >
            {t('create_new_character')}
          </button>
        </div>
      )}
    </div>
  );
}
