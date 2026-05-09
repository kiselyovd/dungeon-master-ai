/**
 * CharFab (M5 P2.14).
 *
 * Bottom-left FAB pill placed inside `.dm-vtt-panel` that opens the
 * CharacterSheet modal. Renders `<icon> <PC name> <hp/maxHp>`.
 *
 * The component itself does not own the modal open/close state - the
 * parent (App.tsx) wires the click handler. The fab is hidden when no
 * character has been created yet (`pc.name === null`).
 */

import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';

interface CharFabProps {
  onOpen: () => void;
}

export function CharFab({ onOpen }: CharFabProps) {
  const { t } = useTranslation('character');
  const name = useStore((s) => s.pc.name);
  const hp = useStore((s) => s.pc.hp);
  const hpMax = useStore((s) => s.pc.hpMax);

  if (name === null) return null;

  return (
    <button
      type="button"
      className="dm-char-fab"
      onClick={onOpen}
      aria-label={t('fab_aria', { name })}
    >
      <Icons.User size={14} />
      <span className="dm-char-fab-name">{name}</span>
      <span className="dm-char-fab-hp dm-mono">
        {hp}/{hpMax}
      </span>
    </button>
  );
}
