import {
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import tokenCleric from '../assets/token-cleric.png';
import tokenFighter from '../assets/token-fighter.png';
import tokenRogue from '../assets/token-rogue.png';
import tokenWizard from '../assets/token-wizard.png';
import type { CombatToken as TokenData } from '../state/combat';
import { useStore } from '../state/useStore';

const CLASS_TOKEN: Record<string, string> = {
  fighter: tokenFighter,
  wizard: tokenWizard,
  rogue: tokenRogue,
  cleric: tokenCleric,
};

interface Props {
  token: TokenData;
  cellSize: number;
  /** Viewport scale; screen-px drag deltas divide by this to reach world px. */
  zoom?: number;
  onMove?: (id: string, x: number, y: number) => void;
  /** The id of the token whose turn it currently is. Used to gate dragging. */
  currentTurnId?: string | null | undefined;
}

interface DragSession {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  originCol: number;
  originRow: number;
}

function hpColor(hp: number, maxHp: number): string {
  const pct = maxHp > 0 ? hp / maxHp : 0;
  if (pct > 0.5) return 'var(--color-success)';
  if (pct > 0.25) return 'var(--color-warning)';
  return 'var(--color-danger)';
}

const CONDITION_COLORS: Record<string, string> = {
  poisoned: 'var(--magic-necromancy)',
  stunned: 'var(--magic-enchantment)',
  prone: 'var(--color-fg-muted)',
  unconscious: 'var(--color-danger)',
};

function conditionColor(condition: string): string {
  return CONDITION_COLORS[condition] ?? 'var(--color-warning)';
}

/**
 * CombatToken renders as a positioned HTML overlay above the VTT canvas.
 * Using HTML/CSS rather than PixiJS primitives keeps the token testable in
 * jsdom and lets the design tokens flow in via CSS custom properties.
 *
 * Design spec:
 * - Pulsing gold ring on active turn: 1.6s scale + opacity loop via CSS.
 * - AC chip overlay: top-right, 18x18 round, mono 11px.
 * - Status-icon ring: max 3 visible + "+N" badge.
 * - HP bar color by percent: success >50%, warning 25-50%, danger <25%.
 *
 * When `onMove` is provided, the token becomes pointer-draggable with grid
 * snap on release and a ghost preview at the origin while the drag is in
 * flight. Without `onMove`, no handlers attach and the token stays static
 * (the "view-only" mode the chat replay and screenshot tooling rely on).
 */
export function CombatToken({ token, cellSize, zoom = 1, onMove, currentTurnId }: Props) {
  const pcName = useStore((s) => s.pc.name);
  const pcHeroClass = useStore((s) => s.pc.heroClass);
  const pcPortraitUrl = useStore((s) => s.pc.portraitUrl);
  const originLeft = token.x * cellSize;
  const originTop = token.y * cellSize;
  const hpPct = token.maxHp > 0 ? (token.hp / token.maxHp) * 100 : 0;
  const visibleConditions = token.conditions.slice(0, 3);
  const extraConditions = token.conditions.length > 3 ? token.conditions.length - 3 : 0;
  const isDead = token.hp === 0;
  const isPcToken = pcName !== null && token.name === pcName;
  // The PC's own token prefers the generated/selected portrait over class art (E4).
  const portraitSrc = isPcToken
    ? (pcPortraitUrl ?? (pcHeroClass !== null ? (CLASS_TOKEN[pcHeroClass] ?? null) : null))
    : null;

  const dragRef = useRef<DragSession | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [liveLeft, setLiveLeft] = useState<number | null>(null);
  const [liveTop, setLiveTop] = useState<number | null>(null);

  const cancelDrag = useCallback(() => {
    dragRef.current = null;
    setIsDragging(false);
    setLiveLeft(null);
    setLiveTop(null);
  }, []);

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      // Primary button only; right/middle/aux clicks must not start a drag
      // so the canvas context menu and middle-pan can layer on later.
      if (event.button !== 0) return;
      event.preventDefault();
      // Stop the canvas pan handler (on the parent .dm-vtt-canvas) from also
      // starting - dragging a token must move the token, not pan the map.
      event.stopPropagation();
      try {
        event.currentTarget.setPointerCapture(event.pointerId);
      } catch {
        // jsdom or stale capture targets may throw; the drag still works
        // without capture because we listen on the same element.
      }
      dragRef.current = {
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startClientY: event.clientY,
        originCol: token.x,
        originRow: token.y,
      };
      setIsDragging(true);
      setLiveLeft(originLeft);
      setLiveTop(originTop);
    },
    [originLeft, originTop, token.x, token.y],
  );

  const onPointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) return;
      // Screen-px delta -> world px: the token lives inside the zoom-scaled
      // world wrapper, so a screen move of D px is D/zoom world px.
      const deltaX = (event.clientX - drag.startClientX) / zoom;
      const deltaY = (event.clientY - drag.startClientY) / zoom;
      setLiveLeft(drag.originCol * cellSize + deltaX);
      setLiveTop(drag.originRow * cellSize + deltaY);
    },
    [cellSize, zoom],
  );

  const onPointerEnd = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) return;
      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {
        // Capture may have been auto-released already (pointercancel path);
        // swallow so cleanup still runs.
      }
      if (event.type !== 'pointercancel' && onMove !== undefined) {
        const deltaX = (event.clientX - drag.startClientX) / zoom;
        const deltaY = (event.clientY - drag.startClientY) / zoom;
        const newCol = Math.max(0, drag.originCol + Math.round(deltaX / cellSize));
        const newRow = Math.max(0, drag.originRow + Math.round(deltaY / cellSize));
        onMove(token.id, newCol, newRow);
      }
      cancelDrag();
    },
    [cancelDrag, cellSize, zoom, onMove, token.id],
  );

  // Escape key cancellation. Listening on window (vs. the token element) so
  // the user can press Escape even after the pointer has wandered off the
  // token bounds mid-drag.
  useEffect(() => {
    if (!isDragging) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') cancelDrag();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isDragging, cancelDrag]);

  // Unmount cleanup so an in-flight drag does not leak the ref across
  // remounts (route change, hot reload, error boundary recovery).
  useEffect(
    () => () => {
      dragRef.current = null;
    },
    [],
  );

  const currentLeft = isDragging && liveLeft !== null ? liveLeft : originLeft;
  const currentTop = isDragging && liveTop !== null ? liveTop : originTop;
  // A token is draggable only when it is the active PC token on its turn and alive.
  const draggable = onMove !== undefined && isPcToken && token.id === currentTurnId && !isDead;

  return (
    <>
      {isDragging && (
        <div
          data-ghost="true"
          aria-hidden="true"
          style={{
            position: 'absolute',
            left: originLeft,
            top: originTop,
            width: cellSize,
            height: cellSize,
            borderRadius: '50%',
            background: 'var(--color-bg-raised)',
            border: '2px dashed var(--color-accent)',
            opacity: 0.3,
            pointerEvents: 'none',
          }}
        />
      )}
      <div
        data-testid={`combat-token-${token.id}`}
        data-active={token.isActive ? 'true' : undefined}
        data-dead={isDead ? 'true' : undefined}
        onPointerDown={draggable ? onPointerDown : undefined}
        onPointerMove={draggable ? onPointerMove : undefined}
        onPointerUp={draggable ? onPointerEnd : undefined}
        onPointerCancel={draggable ? onPointerEnd : undefined}
        style={{
          position: 'absolute',
          left: currentLeft,
          top: currentTop,
          width: cellSize,
          height: cellSize,
          // Pulse pauses during drag so the active-token glow does not
          // visually compete with the ghost + live position feedback. Dead
          // tokens never pulse - the skull overlay is the only motion cue.
          animation:
            token.isActive && !isDragging && !isDead
              ? 'token-pulse 1.6s ease-in-out infinite'
              : undefined,
          cursor: draggable ? (isDragging ? 'grabbing' : 'grab') : undefined,
          zIndex: isDragging ? 10 : undefined,
          touchAction: 'none',
          filter: isDead ? 'grayscale(100%)' : undefined,
          opacity: isDead ? 0.7 : undefined,
        }}
      >
        <div
          style={{
            width: cellSize - 4,
            height: cellSize - 4,
            borderRadius: '50%',
            background: 'var(--color-bg-raised)',
            border: token.isActive
              ? '2px solid var(--color-accent)'
              : '1px solid var(--color-border-subtle)',
            boxShadow: token.isActive ? 'var(--glow-accent)' : undefined,
            position: 'relative',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 'var(--text-xs)',
            color: 'var(--color-fg-primary)',
            overflow: 'hidden',
            backgroundImage: portraitSrc !== null ? `url(${portraitSrc})` : undefined,
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
        >
          {portraitSrc === null && token.name.charAt(0).toUpperCase()}

          {isDead && (
            <div
              data-testid="combat-token-skull"
              role="img"
              aria-label="Dead"
              style={{
                position: 'absolute',
                inset: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: Math.floor(cellSize * 0.4),
                pointerEvents: 'none',
                color: 'var(--color-fg-secondary)',
              }}
            >
              {'☠'}
            </div>
          )}

          <div
            data-testid={`combat-token-${token.id}-ac`}
            style={{
              position: 'absolute',
              top: -4,
              right: -4,
              width: 18,
              height: 18,
              borderRadius: '50%',
              background: 'var(--color-bg-deep)',
              border: '1px solid var(--color-border-strong)',
              fontSize: 'var(--text-xs)',
              fontFamily: 'var(--font-mono)',
              letterSpacing: 'var(--tracking-mono)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--color-fg-secondary)',
            }}
          >
            {token.ac}
          </div>

          {visibleConditions.length > 0 && (
            <div
              style={{
                position: 'absolute',
                bottom: -8,
                left: 0,
                right: 0,
                display: 'flex',
                justifyContent: 'center',
                gap: 2,
              }}
            >
              {visibleConditions.map((c) => (
                <span
                  key={c}
                  role="img"
                  aria-label={c}
                  title={c}
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    background: conditionColor(c),
                  }}
                />
              ))}
              {extraConditions > 0 && (
                <span
                  style={{
                    fontSize: 8,
                    color: 'var(--color-fg-muted)',
                    lineHeight: '8px',
                  }}
                >
                  +{extraConditions}
                </span>
              )}
            </div>
          )}
        </div>

        <div
          style={{
            position: 'absolute',
            bottom: 0,
            left: 0,
            right: 0,
            height: 3,
            background: 'var(--color-bg-deep)',
            borderRadius: 2,
            overflow: 'hidden',
          }}
        >
          <div
            data-testid={`combat-token-${token.id}-hpbar`}
            style={{
              width: `${hpPct}%`,
              height: '100%',
              background: hpColor(token.hp, token.maxHp),
              transition: 'width var(--t-base) ease',
            }}
          />
        </div>
      </div>
    </>
  );
}
