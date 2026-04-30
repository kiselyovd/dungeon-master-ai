import { useTranslation } from 'react-i18next';

interface Props {
  actionUsed: boolean;
  bonusUsed: boolean;
  reactionUsed: boolean;
  movementFt: number;
  speedFt: number;
  visible: boolean;
  onEndTurn: () => void;
  onActionClick?: () => void;
  onBonusClick?: () => void;
  onReactionClick?: () => void;
}

/**
 * ActionBar slides up from the bottom when it is the player's turn.
 * Transition is `transform: translateX(-50%) translateY(0)` over `var(--t-slow)`.
 */
export function ActionBar({
  actionUsed,
  bonusUsed,
  reactionUsed,
  movementFt,
  visible,
  onEndTurn,
  onActionClick,
  onBonusClick,
  onReactionClick,
}: Props) {
  const { t } = useTranslation('combat');

  return (
    <div className={`action-bar${visible ? ' visible' : ''}`}>
      <button
        type="button"
        data-testid="action-btn-action"
        className={`action-btn${actionUsed ? ' used' : ''}`}
        disabled={actionUsed}
        onClick={onActionClick}
        title={t('action')}
      >
        <span style={{ fontSize: 18 }}>{'⚔'}</span>
        <span style={{ fontSize: 'var(--text-xs)' }}>{t('action')}</span>
      </button>

      <button
        type="button"
        data-testid="action-btn-bonus"
        className={`action-btn${bonusUsed ? ' used' : ''}`}
        disabled={bonusUsed}
        onClick={onBonusClick}
        title={t('bonus_action')}
      >
        <span style={{ fontSize: 18 }}>{'✦'}</span>
        <span style={{ fontSize: 'var(--text-xs)' }}>{t('bonus_action')}</span>
      </button>

      <button
        type="button"
        data-testid="action-btn-reaction"
        className={`action-btn${reactionUsed ? ' used' : ''}`}
        disabled={reactionUsed}
        onClick={onReactionClick}
        title={t('reaction')}
      >
        <span style={{ fontSize: 18 }}>{'⚡'}</span>
        <span style={{ fontSize: 'var(--text-xs)' }}>{t('reaction')}</span>
      </button>

      <button
        type="button"
        data-testid="action-btn-move"
        className="action-btn"
        title={t('move')}
        style={{ opacity: movementFt === 0 ? 0.4 : 1 }}
      >
        <span style={{ fontSize: 18 }}>{'↑'}</span>
        <span style={{ fontSize: 'var(--text-xs)' }}>{movementFt}ft</span>
      </button>

      <button
        type="button"
        data-testid="action-btn-end-turn"
        className="action-btn"
        onClick={onEndTurn}
        style={{
          borderColor: 'var(--color-accent)',
          color: 'var(--color-accent)',
          marginLeft: 'var(--space-2)',
        }}
      >
        <span style={{ fontSize: 'var(--text-xs)' }}>{t('end_turn')}</span>
      </button>
    </div>
  );
}
