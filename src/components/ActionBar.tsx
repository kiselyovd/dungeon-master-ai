import type { ReactElement } from 'react';
import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';

interface Props {
  actionUsed?: boolean;
  bonusUsed?: boolean;
  reactionUsed?: boolean;
  movementFt?: number;
  speedFt?: number;
  onAttack?: (() => void) | undefined;
  onCast?: (() => void) | undefined;
  onMove?: (() => void) | undefined;
  onDash?: (() => void) | undefined;
  onDodge?: (() => void) | undefined;
  onDisengage?: (() => void) | undefined;
  onUseObject?: (() => void) | undefined;
  onEndTurn?: (() => void) | undefined;
  /**
   * Posts a localized action intent to the agent (e.g. "I attack."). When set,
   * each standard-action button both consumes the action economy AND sends the
   * intent so the DM resolves it. [M11 F1]
   */
  onIntent?: ((text: string) => void) | undefined;
}

interface ActionButton {
  key: string;
  label: string;
  title: string;
  icon: ReactElement;
  kind?: 'primary' | 'magic' | 'end' | undefined;
  kbd: string;
  disabled?: boolean | undefined;
  onClick?: (() => void) | undefined;
}

/**
 * ActionBar overlay shown bottom-center over the VTT during the player's turn.
 * Renders the 8 standard actions + economy chips per the design brief.
 *
 * All economy/handler props are optional: when omitted, the bar reads from
 * the CombatSlice (and pc.speedFt) so it can be mounted as bare `<ActionBar />`.
 * Explicit props always win via `??` so the existing override path still works.
 */
export function ActionBar({
  actionUsed: actionUsedProp,
  bonusUsed: bonusUsedProp,
  reactionUsed: reactionUsedProp,
  movementFt: movementFtProp,
  speedFt: speedFtProp,
  onAttack,
  onCast,
  onMove,
  onDash,
  onDodge,
  onDisengage,
  onUseObject,
  onEndTurn,
  onIntent,
}: Props) {
  const { t } = useTranslation('combat');

  const storeActionUsed = useStore((s) => s.combat.actionUsed);
  const storeBonusUsed = useStore((s) => s.combat.bonusUsed);
  const storeReactionUsed = useStore((s) => s.combat.reactionUsed);
  const storeMovementRemaining = useStore((s) => s.combat.movementRemaining);
  const storeSpeedFt = useStore((s) => s.pc.speedFt);
  const storeEndTurn = useStore((s) => s.combat.endTurn);
  const storeUseAction = useStore((s) => s.combat.useAction);

  const actionUsed = actionUsedProp ?? storeActionUsed;
  const bonusUsed = bonusUsedProp ?? storeBonusUsed;
  const reactionUsed = reactionUsedProp ?? storeReactionUsed;
  const movementFt = movementFtProp ?? storeMovementRemaining;
  const speedFt = speedFtProp ?? storeSpeedFt;
  const resolvedEndTurn = onEndTurn ?? storeEndTurn;

  // An action button (when not explicitly overridden) consumes the action
  // economy from the store and posts a localized intent to the DM agent. [F1]
  const actionHandler = (explicit: (() => void) | undefined, intentKey: string): (() => void) =>
    explicit ??
    (() => {
      storeUseAction();
      onIntent?.(t(intentKey));
    });
  // Move has no action-economy cost here (movement is consumed by token drag);
  // it just posts the intent when bare.
  const moveHandler = onMove ?? (() => onIntent?.(t('intent_move')));

  const moveLabel = `${t('move')} (${t('movement_label', { remaining: movementFt, total: speedFt })})`;
  const actionUsedTitle = t('action_used_tooltip');

  const actions: ActionButton[] = [
    {
      key: 'attack',
      label: t('attack'),
      title: actionUsed ? actionUsedTitle : t('attack'),
      icon: <Icons.Sword size={20} />,
      kind: 'primary',
      kbd: 'A',
      disabled: actionUsed,
      onClick: actionHandler(onAttack, 'intent_attack'),
    },
    {
      key: 'cast',
      label: t('cast'),
      title: actionUsed ? actionUsedTitle : t('cast'),
      icon: <Icons.Wand size={20} />,
      kind: 'magic',
      kbd: 'C',
      disabled: actionUsed,
      onClick: actionHandler(onCast, 'intent_cast'),
    },
    {
      key: 'move',
      label: moveLabel,
      title: moveLabel,
      icon: <Icons.Footprints size={20} />,
      kbd: 'M',
      disabled: movementFt === 0,
      onClick: moveHandler,
    },
    {
      key: 'dash',
      label: t('dash'),
      title: actionUsed ? actionUsedTitle : t('dash'),
      icon: <Icons.Run size={20} />,
      kbd: 'D',
      disabled: actionUsed,
      onClick: actionHandler(onDash, 'intent_dash'),
    },
    {
      key: 'dodge',
      label: t('dodge'),
      title: actionUsed ? actionUsedTitle : t('dodge'),
      icon: <Icons.ShieldHalf size={20} />,
      kbd: 'V',
      disabled: actionUsed,
      onClick: actionHandler(onDodge, 'intent_dodge'),
    },
    {
      key: 'disengage',
      label: t('disengage'),
      title: actionUsed ? actionUsedTitle : t('disengage'),
      icon: <Icons.ArrowReverse size={20} />,
      kbd: 'X',
      disabled: actionUsed,
      onClick: actionHandler(onDisengage, 'intent_disengage'),
    },
    {
      key: 'use_object',
      label: t('use_object'),
      title: actionUsed ? actionUsedTitle : t('use_object'),
      icon: <Icons.Hand size={20} />,
      kbd: 'U',
      disabled: actionUsed,
      onClick: actionHandler(onUseObject, 'intent_use_object'),
    },
    {
      key: 'end_turn',
      label: t('end_turn'),
      title: t('end_turn'),
      icon: <Icons.Hourglass size={20} />,
      kind: 'end',
      kbd: 'Enter',
      disabled: resolvedEndTurn === undefined,
      onClick: resolvedEndTurn,
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
            title={a.title}
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
