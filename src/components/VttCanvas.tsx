import { Application, extend } from '@pixi/react';
import type { Graphics as PixiGraphics } from 'pixi.js';
import { Container, Graphics } from 'pixi.js';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';
import { CombatOverlay } from './CombatOverlay';

extend({ Container, Graphics });

interface Props {
  /** Optional override; when omitted the count is derived from the container width. */
  widthCells?: number;
  /** Optional override; when omitted the count is derived from the container height. */
  heightCells?: number;
  cellSize?: number;
}

/** Minimum pixel size we let the Pixi viewport shrink to so we never render at 0x0. */
const MIN_CANVAS_PX = 180;
/** Minimum cells along either axis (keeps the grid sensible on tiny containers). */
const MIN_CELLS = 6;
/** Fallback dimensions when no ResizeObserver and no container size are available. */
const FALLBACK_CELLS = 20;

function deriveCells(containerPx: number, cellSize: number, override: number | undefined): number {
  if (override && override > 0) return override;
  if (containerPx <= 0) return FALLBACK_CELLS;
  return Math.max(MIN_CELLS, Math.floor(containerPx / cellSize));
}

export function VttCanvas({ widthCells, heightCells, cellSize = 30 }: Props) {
  const { t } = useTranslation('combat');

  const combatActive = useStore((s) => s.combat.active);
  const tokens = useStore((s) => s.combat.tokens);
  const moveToken = useStore((s) => s.combat.moveToken);
  const [showGrid, setShowGrid] = useState(true);
  const [measureMode, setMeasureMode] = useState(false);

  // Container-driven canvas size. Initialised to defaults so the very first
  // render before ResizeObserver fires still produces a usable grid.
  const [containerSize, setContainerSize] = useState<{ width: number; height: number }>(() => ({
    width: FALLBACK_CELLS * cellSize,
    height: FALLBACK_CELLS * cellSize,
  }));

  const containerRef = useRef<HTMLDivElement | null>(null);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const applySize = (w: number, h: number) => {
      setContainerSize((prev) => {
        if (prev.width === w && prev.height === h) return prev;
        return { width: w, height: h };
      });
    };

    const scheduleApply = (w: number, h: number) => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
      }
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        applySize(w, h);
      });
    };

    // Seed once synchronously from the current layout box so the grid does
    // not flash at the fallback size on mount.
    applySize(el.clientWidth, el.clientHeight);

    // Modern path: observe container with ResizeObserver, batch via rAF.
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver((entries) => {
        const entry = entries[0];
        if (!entry) return;
        const cr = entry.contentRect;
        scheduleApply(cr.width, cr.height);
      });
      observer.observe(el);
      return () => {
        observer.disconnect();
        if (rafRef.current !== null) {
          cancelAnimationFrame(rafRef.current);
          rafRef.current = null;
        }
      };
    }

    // Fallback for environments without ResizeObserver: respond to window
    // resize and read clientWidth/clientHeight directly.
    const onWindowResize = () => {
      if (!containerRef.current) return;
      scheduleApply(containerRef.current.clientWidth, containerRef.current.clientHeight);
    };
    window.addEventListener('resize', onWindowResize);
    return () => {
      window.removeEventListener('resize', onWindowResize);
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, []);

  // Pixi viewport fills the container so the grid never leaves a dark margin
  // around it. The grid line count is derived independently from the floor of
  // (containerPx / cellSize) so cells stay square even on non-multiple sizes.
  const width = Math.max(MIN_CANVAS_PX, containerSize.width);
  const height = Math.max(MIN_CANVAS_PX, containerSize.height);

  const effectiveWidthCells = deriveCells(width, cellSize, widthCells);
  const effectiveHeightCells = deriveCells(height, cellSize, heightCells);

  const drawGrid = useCallback(
    (g: PixiGraphics) => {
      g.clear();
      g.rect(0, 0, width, height).fill({ color: 0x1a1424, alpha: 1 });
      for (let x = 0; x <= effectiveWidthCells; x += 1) {
        const px = x * cellSize;
        g.moveTo(px, 0).lineTo(px, height);
      }
      for (let y = 0; y <= effectiveHeightCells; y += 1) {
        const py = y * cellSize;
        g.moveTo(0, py).lineTo(width, py);
      }
      g.stroke({ color: 0xd4af37, alpha: 0.18, width: 1 });
    },
    [effectiveWidthCells, effectiveHeightCells, cellSize, width, height],
  );

  const isEmpty = tokens.length === 0 && !combatActive;

  return (
    <div className="dm-vtt" ref={containerRef} data-testid="dm-vtt">
      <div className="dm-vtt-canvas">
        <Application width={width} height={height} backgroundColor={0x14101a}>
          <pixiContainer>
            <pixiGraphics draw={drawGrid} />
          </pixiContainer>
        </Application>
        <CombatOverlay
          active={combatActive}
          tokens={tokens}
          cellSize={cellSize}
          widthCells={effectiveWidthCells}
          heightCells={effectiveHeightCells}
          onMoveToken={moveToken}
        />
      </div>

      {isEmpty && (
        <div className="dm-vtt-empty" aria-live="polite">
          <Icons.Map size={48} className="dm-vtt-empty-icon" />
          <div className="dm-vtt-empty-title">{t('empty_map_title')}</div>
          <div className="dm-vtt-empty-text">{t('empty_map_text')}</div>
        </div>
      )}

      <div className="dm-vtt-scale" aria-hidden="true">
        <div className="dm-vtt-scale-bar" />
        <span>5 ft</span>
      </div>

      <div className="dm-vtt-controls" role="toolbar" aria-label="Map controls">
        <button type="button" className="dm-vtt-ctrl" title="Zoom in" disabled>
          <Icons.ZoomIn size={16} />
        </button>
        <button type="button" className="dm-vtt-ctrl" title="Zoom out" disabled>
          <Icons.ZoomOut size={16} />
        </button>
        <button type="button" className="dm-vtt-ctrl" title="Fit to view" disabled>
          <Icons.Maximize size={16} />
        </button>
        <div className="dm-vtt-ctrl-divider" />
        <button
          type="button"
          className={`dm-vtt-ctrl${showGrid ? ' is-active' : ''}`}
          title="Toggle grid"
          onClick={() => setShowGrid((v) => !v)}
        >
          <Icons.GridIcon size={16} />
        </button>
        <button
          type="button"
          className={`dm-vtt-ctrl${measureMode ? ' is-active' : ''}`}
          title="Measure"
          onClick={() => setMeasureMode((v) => !v)}
        >
          <Icons.Ruler size={16} />
        </button>
        <button type="button" className="dm-vtt-ctrl" title="Layers" disabled>
          <Icons.Layers size={16} />
        </button>
      </div>
    </div>
  );
}
