import { useEffect } from 'react';
import type { AoeTemplateEntry, CombatToken } from '../state/combat';
import { useStore } from '../state/useStore';
import { AoeTemplate } from './AoeTemplate';
import { CombatToken as CombatTokenComponent } from './CombatToken';

interface Props {
  active: boolean;
  tokens: CombatToken[];
  cellSize: number;
  /** Current viewport scale, so token drag deltas (screen px) map to world px. */
  zoom?: number;
  widthCells: number;
  heightCells: number;
  onMoveToken?: (id: string, x: number, y: number) => void;
  aoeTemplates?: AoeTemplateEntry[];
}

/**
 * CombatOverlay sits above the PixiJS grid. It renders HTML token overlays
 * positioned absolutely over the canvas. The 280ms cross-fade combat-entry
 * transition is driven by the .vtt-combat-overlay + .active CSS classes
 * (var(--t-slow)) defined in src/styles/combat.css.
 *
 * Also renders AoE templates with auto-expiry based on each entry's
 * expiresAt timestamp. Templates already past their expiry are removed
 * immediately; the rest are scheduled via setTimeout and cleared on unmount.
 */
export function CombatOverlay({
  active,
  tokens,
  cellSize,
  zoom = 1,
  widthCells,
  heightCells,
  onMoveToken,
  aoeTemplates = [],
}: Props) {
  const width = widthCells * cellSize;
  const height = heightCells * cellSize;

  const removeAoeTemplate = useStore((s) => s.combat.removeAoeTemplate);

  useEffect(() => {
    if (aoeTemplates.length === 0) return;
    const timeouts: ReturnType<typeof setTimeout>[] = [];
    const now = Date.now();
    for (const tmpl of aoeTemplates) {
      const remaining = tmpl.expiresAt - now;
      if (remaining <= 0) {
        removeAoeTemplate(tmpl.id);
        continue;
      }
      timeouts.push(
        setTimeout(() => {
          removeAoeTemplate(tmpl.id);
        }, remaining),
      );
    }
    return () => {
      for (const id of timeouts) clearTimeout(id);
    };
  }, [aoeTemplates, removeAoeTemplate]);

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
          zoom={zoom}
          {...(onMoveToken ? { onMove: onMoveToken } : {})}
        />
      ))}
      {aoeTemplates.map((tmpl) => (
        <AoeTemplate
          key={tmpl.id}
          shape={tmpl.shape}
          originX={tmpl.originX}
          originY={tmpl.originY}
          cellSize={cellSize}
          sizeInFt={tmpl.sizeInFt}
          rotateDeg={tmpl.rotateDeg}
        />
      ))}
    </div>
  );
}
