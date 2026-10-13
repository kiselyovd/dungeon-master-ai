import {
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import type { MediaReference, MediaScope } from '../api/contracts/mediaAsset';
import { isDeadToken, isIncapacitatedToken } from '../features/combat/model/selectors';
import { useMediaImage } from '../features/media/useMediaImage';
import { legacyPortraitUrl } from '../lib/heroPortraits';
import type { CombatToken as TokenData } from '../state/combat';
import { useStore } from '../state/useStore';
import { type ActorVisualPresentation, actorBuiltinAppearance } from './actorVisuals';

export interface PlayerTokenPresentation {
  characterId: string | null;
  heroClass: string | null;
  portraitUrl: string | null;
  portraitAssetId?: string | null;
  portraitMediaRef?: MediaReference | null;
  mediaScope?: MediaScope | null;
}

interface Props {
  selected?: boolean | undefined;
  onSelect?: ((id: string) => void) | undefined;
  actorVisual?: ActorVisualPresentation | undefined;
  playerPresentation?: PlayerTokenPresentation;
  revision?: number;
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
export function CombatToken({
  selected,
  onSelect,
  actorVisual,
  token,
  cellSize,
  zoom = 1,
  onMove,
  currentTurnId,
  playerPresentation,
  revision,
}: Props) {
  const { t } = useTranslation('combat');
  const displayName = token.displayNameKey
    ? t(token.displayNameKey, { defaultValue: token.name })
    : token.name;
  const storedHeroClass = useStore((s) => s.pc.heroClass);
  const pcHeroClass = playerPresentation ? playerPresentation.heroClass : storedHeroClass;
  const storedPortrait = useStore((s) => s.pc.portraitUrl);
  const storedPortraitAsset = useStore((s) => s.pc.portraitAssetId);
  const storedMedia = useStore((s) => s.pc.portraitMediaRef);
  const campaignId = useStore((s) => s.session.activeCampaignId);
  const portraitMediaRef = playerPresentation ? playerPresentation.portraitMediaRef : storedMedia;
  const mediaScope = playerPresentation
    ? (playerPresentation.mediaScope ?? null)
    : campaignId
      ? { kind: 'campaign' as const, campaign_id: campaignId }
      : null;
  const portraitAssetId = playerPresentation
    ? playerPresentation.portraitAssetId
    : storedPortraitAsset;
  const pcPortraitUrl = playerPresentation ? playerPresentation.portraitUrl : storedPortrait;
  const originLeft = token.x * cellSize;
  const originTop = token.y * cellSize;
  const hpPct = token.maxHp > 0 ? (token.hp / token.maxHp) * 100 : 0;
  const visibleConditions = token.conditions.slice(0, 3);
  const extraConditions = token.conditions.length > 3 ? token.conditions.length - 3 : 0;
  const isDead = isDeadToken(token);
  const incapacitated = isIncapacitatedToken(token);
  const storedCharacterId = useStore((s) => s.pc.characterId);
  const pcCharacterId = playerPresentation ? playerPresentation.characterId : storedCharacterId;
  const isPcToken = Boolean(pcCharacterId && token.characterId === pcCharacterId);
  const visual: ActorVisualPresentation | null =
    actorVisual ??
    (isPcToken
      ? {
          characterId: pcCharacterId ?? null,
          heroClass: pcHeroClass,
          portraitAssetId: portraitAssetId ?? null,
          portraitMediaRef: portraitMediaRef ?? null,
          portraitUrl: pcPortraitUrl,
          mediaScope,
        }
      : null);
  const builtin = visual ? actorBuiltinAppearance(visual) : null;
  const media = useMediaImage(
    visual?.portraitMediaRef ?? null,
    visual?.mediaScope ?? null,
    'portrait',
  );
  const miniatureSrc = builtin?.miniature ?? null;
  // Respect the OS-level "prefers-reduced-motion" setting so animations do not
  // play for users who have requested less motion. Checked once at render time;
  // does not need to be reactive since the media-query match value is stable for
  // the lifetime of a token render. Guards against jsdom (which lacks matchMedia)
  // by checking the method exists before calling it. [W1.7b]
  const prefersReducedMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  // The PC's own token prefers the generated/selected portrait over class art (E4).
  const portraitSrc = visual?.portraitMediaRef
    ? media.url
    : (builtin?.portrait ?? (visual?.portraitUrl ? legacyPortraitUrl(visual.portraitUrl) : null));

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
    (event: ReactPointerEvent<HTMLButtonElement>) => {
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
    (event: ReactPointerEvent<HTMLButtonElement>) => {
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
    (event: ReactPointerEvent<HTMLButtonElement>) => {
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
  const draggable =
    onMove !== undefined && isPcToken && token.id === currentTurnId && !incapacitated;
  useEffect(() => {
    if (!draggable) cancelDrag();
  }, [draggable, cancelDrag]);
  const dragAuthority = useRef({
    id: token.id,
    x: token.x,
    y: token.y,
    turn: currentTurnId,
    character: pcCharacterId,
    revision,
  });
  useEffect(() => {
    const old = dragAuthority.current;
    if (
      old.id !== token.id ||
      old.x !== token.x ||
      old.y !== token.y ||
      old.turn !== currentTurnId ||
      old.character !== pcCharacterId ||
      old.revision !== revision
    )
      cancelDrag();
    dragAuthority.current = {
      id: token.id,
      x: token.x,
      y: token.y,
      turn: currentTurnId,
      character: pcCharacterId,
      revision,
    };
  }, [token.id, token.x, token.y, currentTurnId, pcCharacterId, revision, cancelDrag]);

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
            background: miniatureSrc ? 'transparent' : 'var(--color-bg-raised)',
            border: '2px dashed var(--color-accent)',
            opacity: 0.3,
            pointerEvents: 'none',
          }}
        />
      )}
      <button
        data-testid={`combat-token-${token.id}`}
        type="button"
        data-selected={selected ? 'true' : undefined}
        tabIndex={onSelect ? 0 : -1}
        aria-disabled={!onSelect}
        aria-label={t('token_aria', {
          name: displayName,
          hp: token.hp,
          maxHp: token.maxHp,
          ac: token.ac,
        })}
        aria-pressed={Boolean(selected)}
        onClick={
          onSelect
            ? (event) => {
                event.stopPropagation();
                onSelect(token.id);
              }
            : undefined
        }
        onKeyDown={
          onSelect
            ? (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  event.stopPropagation();
                  onSelect(token.id);
                }
              }
            : undefined
        }
        title={displayName}
        data-active={token.isActive ? 'true' : undefined}
        data-dead={isDead ? 'true' : undefined}
        data-downed={token.hp <= 0 && !isDead ? 'true' : undefined}
        onPointerDown={draggable ? onPointerDown : undefined}
        onPointerMove={draggable ? onPointerMove : undefined}
        onPointerUp={draggable ? onPointerEnd : undefined}
        onPointerCancel={draggable ? onPointerEnd : undefined}
        style={{
          padding: 0,
          border: 0,
          background: 'transparent',
          textAlign: 'inherit',
          position: 'absolute',
          left: currentLeft,
          top: currentTop,
          width: cellSize,
          height: cellSize,
          outline: selected ? '2px solid var(--color-accent-soft)' : undefined,
          outlineOffset: selected ? 2 : undefined,
          // Animate model-driven and drop-committed moves (left/top change when
          // isDragging=false). Suppress transition during live drag so the token
          // tracks the pointer with zero lag. [W1.7a]
          transition: isDragging ? undefined : 'left 0.25s ease, top 0.25s ease',
          // Pulse pauses during drag so the active-token glow does not
          // visually compete with the ghost + live position feedback. Dead
          // tokens never pulse - the skull overlay is the only motion cue.
          // Suppressed when the user prefers reduced motion. [W1.7b]
          animation:
            token.isActive && !isDragging && !incapacitated && !prefersReducedMotion
              ? 'token-pulse 1.6s ease-in-out infinite'
              : undefined,
          cursor: draggable ? (isDragging ? 'grabbing' : 'grab') : undefined,
          zIndex: isDragging ? 10000 : Math.round(token.y * 10 + 10),
          touchAction: 'none',
          filter: isDead ? 'grayscale(100%)' : undefined,
          opacity: isDead ? 0.7 : undefined,
        }}
      >
        {miniatureSrc && (
          <div
            data-testid={`miniature-${token.id}`}
            data-asset-id={builtin?.miniatureId ?? ''}
            style={{
              position: 'absolute',
              left: '50%',
              bottom: cellSize * 0.08,
              width: cellSize * 1.65,
              height: cellSize * 2.15,
              transform: 'translateX(-50%)',
              pointerEvents: 'none',
              zIndex: 2,
            }}
          >
            <img
              src={miniatureSrc}
              alt=""
              draggable={false}
              style={{
                width: '100%',
                height: '100%',
                objectFit: 'contain',
                objectPosition: 'center bottom',
                filter: 'drop-shadow(0 4px 3px rgba(0,0,0,.5))',
                pointerEvents: 'none',
              }}
            />
          </div>
        )}
        <div
          style={{
            width: cellSize - 4,
            height: cellSize - 4,
            borderRadius: '50%',
            background: miniatureSrc ? 'transparent' : 'var(--color-bg-raised)',
            marginTop: miniatureSrc ? cellSize * 0.2 : undefined,
            // PC active turn: thicker gold ring + outer glow ring to make it
            // unmistakable at a glance. Enemy/NPC active: standard accent ring.
            // [W1.7b]
            border:
              token.isActive && isPcToken
                ? `${miniatureSrc ? 1 : 3}px solid var(--color-accent)`
                : token.isActive
                  ? '2px solid var(--color-accent)'
                  : '1px solid var(--color-border-subtle)',
            boxShadow:
              token.isActive && isPcToken
                ? miniatureSrc
                  ? 'var(--glow-accent)'
                  : '0 0 0 2px var(--color-accent), var(--glow-accent)'
                : token.isActive
                  ? 'var(--glow-accent)'
                  : undefined,
            position: 'relative',
            zIndex: 3,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 'var(--text-xs)',
            color: 'var(--color-fg-primary)',
            overflow: miniatureSrc ? 'visible' : 'hidden',
            backgroundImage:
              !miniatureSrc && portraitSrc !== null ? `url(${portraitSrc})` : undefined,
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
        >
          {!miniatureSrc && portraitSrc === null && (
            <svg
              aria-hidden="true"
              width={cellSize * 0.65}
              height={cellSize * 0.65}
              viewBox="0 0 32 32"
              style={{ pointerEvents: 'none' }}
            >
              <circle cx="16" cy="9" r="5" fill="var(--color-accent-soft)" />
              <path d="M10 15h12l3 12H7z" fill="var(--color-fg-muted)" />
            </svg>
          )}

          {isDead && (
            <div
              data-testid="combat-token-skull"
              role="img"
              aria-label={t('dead')}
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
              right: miniatureSrc ? -cellSize * 0.25 : -4,
              width: 18,
              height: 18,
              borderRadius: '50%',
              background: 'var(--color-bg-deep)',
              border: '1px solid var(--color-border-strong)',
              fontSize: 'var(--text-xs)',
              fontFamily: 'var(--font-body)',
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

        {/* "Your turn" cue: shown only when the PC's own token is active and
            alive. Floats above the token circle so it does not overlap
            conditions or the HP bar. [W1.7b] */}
        {isPcToken && token.isActive && !incapacitated && (
          <div
            data-testid={`combat-token-${token.id}-your-turn`}
            className="token-your-turn"
            aria-live="polite"
            style={{
              position: 'absolute',
              top: -(cellSize * 0.35),
              left: '50%',
              transform: 'translateX(-50%)',
              pointerEvents: 'none',
              whiteSpace: 'nowrap',
              fontSize: Math.max(9, Math.floor(cellSize * 0.22)),
              fontWeight: 700,
              fontFamily: 'var(--font-display)',
              letterSpacing: '0.06em',
              textTransform: 'uppercase',
              color: 'var(--color-accent)',
              textShadow: '0 1px 4px rgba(0,0,0,0.7)',
            }}
          >
            {t('your_turn')}
          </div>
        )}
      </button>
    </>
  );
}
