import { Application, extend } from '@pixi/react';
import type { Graphics as PixiGraphics } from 'pixi.js';
import { Container, Graphics } from 'pixi.js';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';
import { CombatOverlay } from './CombatOverlay';

extend({ Container, Graphics });

interface Props {
  widthCells?: number;
  heightCells?: number;
  cellSize?: number;
}

export function VttCanvas({ widthCells = 20, heightCells = 20, cellSize = 30 }: Props) {
  const width = widthCells * cellSize;
  const height = heightCells * cellSize;
  const { t } = useTranslation('combat');

  const combatActive = useStore((s) => s.combat.active);
  const tokens = useStore((s) => s.combat.tokens);
  const moveToken = useStore((s) => s.combat.moveToken);
  const [showGrid, setShowGrid] = useState(true);
  const [measureMode, setMeasureMode] = useState(false);

  const drawGrid = useCallback(
    (g: PixiGraphics) => {
      g.clear();
      g.rect(0, 0, width, height).fill({ color: 0x1a1424, alpha: 1 });
      for (let x = 0; x <= widthCells; x += 1) {
        const px = x * cellSize;
        g.moveTo(px, 0).lineTo(px, height);
      }
      for (let y = 0; y <= heightCells; y += 1) {
        const py = y * cellSize;
        g.moveTo(0, py).lineTo(width, py);
      }
      g.stroke({ color: 0xd4af37, alpha: 0.18, width: 1 });
    },
    [widthCells, heightCells, cellSize, width, height],
  );

  const isEmpty = tokens.length === 0 && !combatActive;

  return (
    <div className="dm-vtt">
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
          widthCells={widthCells}
          heightCells={heightCells}
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
