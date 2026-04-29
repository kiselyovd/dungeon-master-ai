import type { CombatToken } from '../state/combat';
import { CombatToken as CombatTokenComponent } from './CombatToken';

interface Props {
  active: boolean;
  tokens: CombatToken[];
  cellSize: number;
  widthCells: number;
  heightCells: number;
  onMoveToken?: (id: string, x: number, y: number) => void;
}

/**
 * CombatOverlay sits above the PixiJS grid. It renders HTML token overlays
 * positioned absolutely over the canvas. The 280ms cross-fade combat-entry
 * transition is driven by the .vtt-combat-overlay + .active CSS classes
 * (var(--t-slow)) defined in src/styles/combat.css.
 */
export function CombatOverlay({
  active,
  tokens,
  cellSize,
  widthCells,
  heightCells,
  onMoveToken,
}: Props) {
  const width = widthCells * cellSize;
  const height = heightCells * cellSize;

  return (
    <div
      data-testid="combat-overlay"
      data-active={active ? 'true' : undefined}
      className={`vtt-combat-overlay${active ? ' active' : ''}`}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width,
        height,
      }}
    >
      {tokens.map((token) => (
        <CombatTokenComponent
          key={token.id}
          token={token}
          cellSize={cellSize}
          {...(onMoveToken ? { onMove: onMoveToken } : {})}
        />
      ))}
    </div>
  );
}
