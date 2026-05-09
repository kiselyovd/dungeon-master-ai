import type { ReactElement } from 'react';
import { useTranslation } from 'react-i18next';
import { Icons } from '../ui/Icons';

interface Props {
  actionUsed: boolean;
  bonusUsed: boolean;
  reactionUsed: boolean;
  movementFt: number;
  speedFt: number;
  onAttack?: (() => void) | undefined;
  onCast?: (() => void) | undefined;
  onMove?: (() => void) | undefined;
  onDash?: (() => void) | undefined;
  onDodge?: (() => void) | undefined;
  onDisengage?: (() => void) | undefined;
  onUseObject?: (() => void) | undefined;
  onEndTurn?: (() => void) | undefined;
}

interface ActionButton {
  key: string;
  label: string;
  icon: ReactElement;
  kind?: 'primary' | 'magic' | 'end' | undefined;
  kbd: string;
  disabled?: boolean | undefined;
  onClick?: (() => void) | undefined;
}

/**
 * ActionBar overlay shown bottom-center over the VTT during the player's turn.
 * Renders the 8 standard actions + economy chips per the design brief.
 */
export function ActionBar({
  actionUsed,
  bonusUsed,
  reactionUsed,
  movementFt,
  speedFt,
  onAttack,
  onCast,
  onMove,
  onDash,
  onDodge,
  onDisengage,
  onUseObject,
  onEndTurn,
}: Props) {
  const { t } = useTranslation('combat');

  const moveLabel = `${t('move')} (${movementFt}/${speedFt} ft)`;

  const actions: ActionButton[] = [
    {
      key: 'attack',
      label: t('attack'),
      icon: <Icons.Sword size={20} />,
      kind: 'primary',
      kbd: 'A',
      disabled: actionUsed,
      onClick: onAttack,
    },
    {
      key: 'cast',
      label: t('cast'),
      icon: <Icons.Wand size={20} />,
      kind: 'magic',
      kbd: 'C',
      disabled: actionUsed,
      onClick: onCast,
    },
    {
      key: 'move',
      label: moveLabel,
      icon: <Icons.Footprints size={20} />,
      kbd: 'M',
      disabled: movementFt === 0,
      onClick: onMove,
    },
    {
      key: 'dash',
      label: t('dash'),
      icon: <Icons.Run size={20} />,
      kbd: 'D',
      disabled: actionUsed,
      onClick: onDash,
    },
    {
      key: 'dodge',
      label: t('dodge'),
      icon: <Icons.ShieldHalf size={20} />,
      kbd: 'V',
      disabled: actionUsed,
      onClick: onDodge,
    },
    {
      key: 'disengage',
      label: t('disengage'),
      icon: <Icons.ArrowReverse size={20} />,
      kbd: 'X',
      disabled: actionUsed,
      onClick: onDisengage,
    },
    {
      key: 'use_object',
      label: t('use_object'),
      icon: <Icons.Hand size={20} />,
      kbd: 'U',
      disabled: actionUsed,
      onClick: onUseObject,
    },
    {
      key: 'end_turn',
      label: t('end_turn'),
      icon: <Icons.Hourglass size={20} />,
      kind: 'end',
      kbd: 'Enter',
      disabled: onEndTurn === undefined,
      onClick: onEndTurn,
    },
  ];

  return (
    <div className="dm-actionbar" role="toolbar" aria-label={t('action_bar')}>
      <div className="dm-actionbar-econ">
        <EconChip label={t('action')} used={actionUsed} />
        <EconChip label={t('bonus_action')} used={bonusUsed} />
        <EconChip label={t('reaction')} used={reactionUsed} />
      </div>
      <div className="dm-actionbar-buttons">
        {actions.map((a) => (
          <button
            key={a.key}
            type="button"
            data-testid={`action-btn-${a.key}`}
            className={`dm-action-btn dm-action-${a.kind ?? 'default'}`}
            disabled={a.disabled === true}
            onClick={a.onClick}
            title={a.label}
          >
            <span className="dm-action-icon">{a.icon}</span>
            <span className="dm-action-label">{a.label}</span>
            <span className="dm-kbd">{a.kbd}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

interface ChipProps {
  label: string;
  used: boolean;
}

function EconChip({ label, used }: ChipProps) {
  return (
    <div className={`dm-econ-chip${used ? ' is-used' : ''}`}>
      <span className="dm-econ-dot" />
      <span>{label}</span>
    </div>
  );
}
