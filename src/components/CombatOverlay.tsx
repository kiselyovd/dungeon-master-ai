import { useEffect, useMemo } from 'react';
import { useConfirmedCombatMotion } from '../features/combat/useConfirmedCombatMotion';
import type { AoeTemplateEntry, CombatToken } from '../state/combat';
import { useStore } from '../state/useStore';
import { AoeTemplate } from './AoeTemplate';
import type { ActorVisualMap } from './actorVisuals';
import { CombatToken as CombatTokenComponent, type PlayerTokenPresentation } from './CombatToken';

interface Props {
  events?: readonly unknown[];
  selectedTokenId?: string | null | undefined;
  onSelectToken?: ((id: string) => void) | undefined;
  actorVisuals?: ActorVisualMap | undefined;
  playerPresentation?: PlayerTokenPresentation;
  movableTokenId?: string | null;
  active: boolean;
  tokens: CombatToken[];
  cellSize: number;
  /** Current viewport scale, so token drag deltas (screen px) map to world px. */
  zoom?: number;
  widthCells: number;
  heightCells: number;
  onMoveToken?: (id: string, x: number, y: number) => void;
  aoeTemplates?: AoeTemplateEntry[];
  /** The id of the token whose turn it currently is. Passed to each CombatToken to gate dragging. */
  currentTurnId?: string | null;
  revision?: number;
  round?: number;
  motionScopeKey?: string;
}
const NO_EVENTS: readonly unknown[] = [];

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
  selectedTokenId,
  onSelectToken,
  actorVisuals,
  active,
  tokens,
  playerPresentation,
  movableTokenId,
  cellSize,
  zoom = 1,
  widthCells,
  heightCells,
  onMoveToken,
  aoeTemplates = [],
  currentTurnId,
  revision,
  round = 1,
  motionScopeKey,
  events = NO_EVENTS,
}: Props) {
  const width = widthCells * cellSize;
  const height = heightCells * cellSize;

  const removeAoeTemplate = useStore((s) => s.combat.removeAoeTemplate);
  const storedCharacterId = useStore((s) => s.pc.characterId);
  const characterId = playerPresentation ? playerPresentation.characterId : storedCharacterId;
  const ownActorId =
    tokens.find((token) => characterId && token.characterId === characterId)?.id ?? null;
  const motionFrame = useMemo(
    () => ({
      scopeKey: motionScopeKey ?? '',
      revision: revision ?? -1,
      round,
      active: active && Boolean(motionScopeKey),
      tokens,
      events,
      ownActorId,
      currentTurnId: currentTurnId ?? null,
    }),
    [motionScopeKey, revision, round, active, tokens, events, ownActorId, currentTurnId],
  );
  const motion = useConfirmedCombatMotion(motionFrame);

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
      data-revision={revision}
      data-current-turn={currentTurnId ?? undefined}
      className={`vtt-combat-overlay${active ? ' active' : ''}`}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width,
        height,
        zIndex: 2,
      }}
    >
      {tokens.map((token) => (
        <CombatTokenComponent
          key={token.id}
          token={token}
          selected={selectedTokenId === token.id}
          onSelect={onSelectToken}
          actorVisual={actorVisuals?.[token.characterId ?? token.id]}
          effects={motion.filter((effect) => effect.actorId === token.id)}
          {...(revision !== undefined ? { revision } : {})}
          cellSize={cellSize}
          zoom={zoom}
          currentTurnId={currentTurnId}
          {...(playerPresentation ? { playerPresentation } : {})}
          {...(onMoveToken && (movableTokenId === undefined || movableTokenId === token.id)
            ? { onMove: onMoveToken }
            : {})}
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
