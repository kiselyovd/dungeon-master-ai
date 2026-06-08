import { Application, extend } from '@pixi/react';
import type { Graphics as PixiGraphics } from 'pixi.js';
import { Container, Graphics } from 'pixi.js';
import type {
  CSSProperties,
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
  WheelEvent as ReactWheelEvent,
} from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import vttEmptyArt from '../assets/vtt-empty.png';
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

/** Floor for the Pixi viewport - never render at 0x0; everything else tracks the container. */
const MIN_CANVAS_PX = 60;
/** Fallback dimensions when no ResizeObserver and no container size are available. */
const FALLBACK_CELLS = 20;

/** Viewport zoom bounds. */
const MIN_ZOOM = 0.4;
const MAX_ZOOM = 4;
const clampZoom = (z: number): number => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));

function deriveCells(containerPx: number, cellSize: number, override: number | undefined): number {
  if (override && override > 0) return override;
  if (containerPx <= 0) return FALLBACK_CELLS;
  return Math.max(1, Math.floor(containerPx / cellSize));
}

export function VttCanvas({ widthCells, heightCells, cellSize = 30 }: Props) {
  const { t } = useTranslation('combat');

  const combatActive = useStore((s) => s.combat.active);
  const tokens = useStore((s) => s.combat.tokens);
  const moveToken = useStore((s) => s.combat.moveToken);
  const aoeTemplates = useStore((s) => s.combat.aoeTemplates);
  const mapImageUrl = useStore((s) => s.session.mapImageUrl);
  const hasMap = mapImageUrl !== null;
  const [showGrid, setShowGrid] = useState(true);
  const [layersOpen, setLayersOpen] = useState(false);
  const [showSceneImage, setShowSceneImage] = useState(true);
  const [showTokens, setShowTokens] = useState(true);
  const [measureMode, setMeasureMode] = useState(false);
  const [measureOrigin, setMeasureOrigin] = useState<{ x: number; y: number } | null>(null);
  const [measureCurrent, setMeasureCurrent] = useState<{ x: number; y: number } | null>(null);

  // Viewport transform: the world wrapper (map image + grid + tokens) is
  // translated by `pan` (screen px) and scaled by `zoom`. Screen-space inputs
  // (token drag deltas, measure distance) divide by `zoom` to reach world px.
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const panSession = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    baseX: number;
    baseY: number;
  } | null>(null);
  const [isPanning, setIsPanning] = useState(false);

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

  // The scene image is shown only when one exists AND its layer is enabled.
  // drawGrid paints the dark backdrop whenever the image is not visible.
  const showMapImage = hasMap && showSceneImage;

  const drawGrid = useCallback(
    (g: PixiGraphics) => {
      g.clear();
      // With a visible map image behind the canvas we paint nothing opaque so
      // the scene art shows through; only the grid lines (drawn below) overlay
      // it. When no image is visible we fill the dark backdrop ourselves.
      if (!showMapImage) {
        g.rect(0, 0, width, height).fill({ color: 0x1a1424, alpha: 1 });
      }
      if (!showGrid) return;
      // Vertical lines at 0, cellSize, 2*cellSize, ... and a closing line at
      // `width` so the right-most partial cell is bounded (avoids a dark
      // unstroked strip when width is not a clean multiple of cellSize).
      for (let x = 0; x <= effectiveWidthCells; x += 1) {
        const px = x * cellSize;
        if (px > width) break;
        g.moveTo(px, 0).lineTo(px, height);
      }
      if (effectiveWidthCells * cellSize < width) {
        g.moveTo(width, 0).lineTo(width, height);
      }
      for (let y = 0; y <= effectiveHeightCells; y += 1) {
        const py = y * cellSize;
        if (py > height) break;
        g.moveTo(0, py).lineTo(width, py);
      }
      if (effectiveHeightCells * cellSize < height) {
        g.moveTo(0, height).lineTo(width, height);
      }
      g.stroke({ color: 0xd4af37, alpha: 0.18, width: 1 });
    },
    [effectiveWidthCells, effectiveHeightCells, cellSize, width, height, showGrid, showMapImage],
  );

  const onMeasureClick = useCallback(
    (e: ReactMouseEvent<SVGSVGElement>) => {
      const rect = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      if (measureOrigin === null) {
        setMeasureOrigin({ x: px, y: py });
        setMeasureCurrent({ x: px, y: py });
      } else {
        setMeasureOrigin(null);
        setMeasureCurrent(null);
      }
    },
    [measureOrigin],
  );

  const onMeasureMouseMove = useCallback(
    (e: ReactMouseEvent<SVGSVGElement>) => {
      if (measureOrigin === null) return;
      const rect = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
      setMeasureCurrent({ x: e.clientX - rect.left, y: e.clientY - rect.top });
    },
    [measureOrigin],
  );

  useEffect(() => {
    if (!measureMode) {
      setMeasureOrigin(null);
      setMeasureCurrent(null);
      return;
    }
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') {
        setMeasureOrigin(null);
        setMeasureCurrent(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [measureMode]);

  const measureDistanceFt = useMemo(() => {
    if (measureOrigin === null || measureCurrent === null) return null;
    const dx = measureCurrent.x - measureOrigin.x;
    const dy = measureCurrent.y - measureOrigin.y;
    // The measure overlay is screen-space; divide by zoom to get world px.
    return Math.round((Math.sqrt(dx * dx + dy * dy) / zoom / cellSize) * 5);
  }, [measureOrigin, measureCurrent, cellSize, zoom]);

  // --- viewport (zoom / pan / fit) --------------------------------------
  // Zoom by `factor` keeping the world point under (cx, cy) (container-relative
  // screen px) fixed, so the map grows/shrinks around the cursor, not the origin.
  const zoomAt = useCallback((factor: number, cx: number, cy: number) => {
    setZoom((prevZoom) => {
      const next = clampZoom(prevZoom * factor);
      if (next === prevZoom) return prevZoom;
      const ratio = next / prevZoom;
      setPan((p) => ({ x: cx - (cx - p.x) * ratio, y: cy - (cy - p.y) * ratio }));
      return next;
    });
  }, []);

  const zoomFromButton = useCallback(
    (factor: number) => {
      const el = containerRef.current;
      const cx = el ? el.clientWidth / 2 : width / 2;
      const cy = el ? el.clientHeight / 2 : height / 2;
      zoomAt(factor, cx, cy);
    },
    [zoomAt, width, height],
  );

  const fitToView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, []);

  const onWheel = useCallback(
    (e: ReactWheelEvent<HTMLDivElement>) => {
      const rect = e.currentTarget.getBoundingClientRect();
      // Zoom in on scroll up (deltaY < 0). 1.0015^px gives smooth trackpad feel.
      const factor = 1.0015 ** -e.deltaY;
      zoomAt(factor, e.clientX - rect.left, e.clientY - rect.top);
    },
    [zoomAt],
  );

  const onPanPointerDown = useCallback(
    (e: ReactPointerEvent<HTMLDivElement>) => {
      // Only the empty canvas pans; tokens capture their own pointer first, and
      // measure mode owns the pointer via its SVG overlay.
      if (e.button !== 0 || measureMode) return;
      panSession.current = {
        pointerId: e.pointerId,
        startX: e.clientX,
        startY: e.clientY,
        baseX: pan.x,
        baseY: pan.y,
      };
      setIsPanning(true);
      try {
        e.currentTarget.setPointerCapture(e.pointerId);
      } catch {
        // jsdom / stale capture - panning still works via the same element.
      }
    },
    [measureMode, pan.x, pan.y],
  );

  const onPanPointerMove = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    const s = panSession.current;
    if (s === null || s.pointerId !== e.pointerId) return;
    setPan({ x: s.baseX + (e.clientX - s.startX), y: s.baseY + (e.clientY - s.startY) });
  }, []);

  const onPanPointerEnd = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    const s = panSession.current;
    if (s === null || s.pointerId !== e.pointerId) return;
    panSession.current = null;
    setIsPanning(false);
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      // already released on pointercancel
    }
  }, []);

  const worldStyle: CSSProperties = {
    position: 'absolute',
    top: 0,
    left: 0,
    width,
    height,
    transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
    transformOrigin: '0 0',
  };

  const isEmpty = tokens.length === 0 && !combatActive && !hasMap;

  return (
    <div className="dm-vtt" ref={containerRef} data-testid="dm-vtt">
      {/* biome-ignore lint/a11y/noStaticElementInteractions: the canvas is a 2D map surface; wheel-zoom and drag-to-pan are pointer-only interactions with no keyboard analogue, and the toolbar buttons provide accessible zoom/fit controls */}
      <div
        className="dm-vtt-canvas"
        onWheel={onWheel}
        onPointerDown={onPanPointerDown}
        onPointerMove={onPanPointerMove}
        onPointerUp={onPanPointerEnd}
        onPointerCancel={onPanPointerEnd}
        style={{ cursor: isPanning ? 'grabbing' : measureMode ? 'crosshair' : 'grab' }}
      >
        <div className="dm-vtt-world" style={worldStyle}>
          {showMapImage && (
            <img
              src={mapImageUrl ?? undefined}
              alt=""
              className="dm-vtt-map-bg"
              data-testid="dm-vtt-map-bg"
            />
          )}
          {/* The canvas is always transparent; the dark backdrop is painted by
              drawGrid's rect fill (skipped when a map image is present so the
              art shows through). backgroundAlpha is an init-only Pixi option, so
              a constant 0 avoids the "opaque clear hides the map" reactivity bug.
              Keying on hasMap forces a renderer re-init if the map toggles, so a
              late-arriving image is never occluded by a stale opaque clear. */}
          <Application
            key={hasMap ? 'vtt-map' : 'vtt-grid'}
            width={width}
            height={height}
            backgroundColor={0x14101a}
            backgroundAlpha={0}
          >
            <pixiContainer>
              <pixiGraphics draw={drawGrid} />
            </pixiContainer>
          </Application>
          {showTokens && (
            <CombatOverlay
              active={combatActive}
              tokens={tokens}
              cellSize={cellSize}
              zoom={zoom}
              widthCells={effectiveWidthCells}
              heightCells={effectiveHeightCells}
              onMoveToken={moveToken}
              aoeTemplates={aoeTemplates}
            />
          )}
        </div>
        {measureMode && (
          // biome-ignore lint/a11y/useKeyWithClickEvents: Escape cancellation is handled at window level above; click is the only viable input for placing a measurement origin on a 2D map
          <svg
            data-testid="measure-overlay"
            role="application"
            aria-label={t('map_measure')}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width,
              height,
              cursor: measureOrigin === null ? 'crosshair' : 'cell',
              pointerEvents: 'all',
            }}
            onClick={onMeasureClick}
            onMouseMove={onMeasureMouseMove}
          >
            <title>{t('map_measure')}</title>
            {measureOrigin !== null && measureCurrent !== null && (
              <>
                <line
                  x1={measureOrigin.x}
                  y1={measureOrigin.y}
                  x2={measureCurrent.x}
                  y2={measureCurrent.y}
                  stroke="var(--color-accent)"
                  strokeWidth={2}
                  strokeDasharray="6 3"
                />
                <circle
                  cx={measureOrigin.x}
                  cy={measureOrigin.y}
                  r={4}
                  fill="var(--color-accent)"
                />
                {measureDistanceFt !== null && (
                  <text
                    data-testid="measure-tooltip"
                    x={measureCurrent.x + 8}
                    y={measureCurrent.y - 8}
                    fill="var(--color-fg-primary)"
                    fontSize={12}
                    fontFamily="var(--font-mono)"
                  >
                    {measureDistanceFt} ft
                  </text>
                )}
              </>
            )}
          </svg>
        )}
      </div>

      {isEmpty && (
        <div className="dm-vtt-empty" aria-live="polite">
          <img src={vttEmptyArt} alt="" className="dm-vtt-empty-art" />
          <div className="dm-vtt-empty-title">{t('empty_map_title')}</div>
          <div className="dm-vtt-empty-text">{t('empty_map_text')}</div>
        </div>
      )}

      <div className="dm-vtt-scale" aria-hidden="true">
        <div className="dm-vtt-scale-bar" />
        <span>{t('map_scale_5ft')}</span>
      </div>

      <div className="dm-vtt-controls" role="toolbar" aria-label={t('map_controls')}>
        <button
          type="button"
          className="dm-vtt-ctrl"
          title={t('map_zoom_in')}
          aria-label={t('map_zoom_in')}
          onClick={() => zoomFromButton(1.25)}
        >
          <Icons.ZoomIn size={16} />
        </button>
        <button
          type="button"
          className="dm-vtt-ctrl"
          title={t('map_zoom_out')}
          aria-label={t('map_zoom_out')}
          onClick={() => zoomFromButton(0.8)}
        >
          <Icons.ZoomOut size={16} />
        </button>
        <button
          type="button"
          className="dm-vtt-ctrl"
          title={t('map_fit_to_view')}
          aria-label={t('map_fit_to_view')}
          onClick={fitToView}
        >
          <Icons.Maximize size={16} />
        </button>
        <div className="dm-vtt-ctrl-divider" />
        <button
          type="button"
          className={`dm-vtt-ctrl${showGrid ? ' is-active' : ''}`}
          title={t('map_toggle_grid')}
          aria-label={t('map_toggle_grid')}
          onClick={() => setShowGrid((v) => !v)}
        >
          <Icons.GridIcon size={16} />
        </button>
        <button
          type="button"
          className={`dm-vtt-ctrl${measureMode ? ' is-active' : ''}`}
          title={t('map_measure')}
          aria-label={t('map_measure')}
          onClick={() => setMeasureMode((v) => !v)}
        >
          <Icons.Ruler size={16} />
        </button>
        <button
          type="button"
          className={`dm-vtt-ctrl${layersOpen ? ' is-active' : ''}`}
          title={t('map_layers')}
          aria-label={t('map_layers')}
          aria-expanded={layersOpen}
          onClick={() => setLayersOpen((v) => !v)}
        >
          <Icons.Layers size={16} />
        </button>
      </div>

      {layersOpen && (
        <div className="dm-vtt-layers" role="menu" aria-label={t('map_layers')}>
          <label className="dm-vtt-layer-row">
            <input type="checkbox" checked={showGrid} onChange={() => setShowGrid((v) => !v)} />
            <span>{t('layer_grid')}</span>
          </label>
          <label className="dm-vtt-layer-row">
            <input
              type="checkbox"
              checked={showSceneImage}
              disabled={!hasMap}
              onChange={() => setShowSceneImage((v) => !v)}
            />
            <span>{t('layer_scene')}</span>
          </label>
          <label className="dm-vtt-layer-row">
            <input type="checkbox" checked={showTokens} onChange={() => setShowTokens((v) => !v)} />
            <span>{t('layer_tokens')}</span>
          </label>
        </div>
      )}
    </div>
  );
}
