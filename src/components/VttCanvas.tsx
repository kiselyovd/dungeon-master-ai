import { useCallback } from 'react';
import { Application, extend } from '@pixi/react';
import { Container, Graphics } from 'pixi.js';
import type { Graphics as PixiGraphics } from 'pixi.js';

extend({ Container, Graphics });

interface Props {
  widthCells?: number;
  heightCells?: number;
  cellSize?: number;
}

export function VttCanvas({ widthCells = 20, heightCells = 20, cellSize = 30 }: Props) {
  const width = widthCells * cellSize;
  const height = heightCells * cellSize;

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

  return (
    <Application width={width} height={height} backgroundColor={0x14101a}>
      <pixiContainer>
        <pixiGraphics draw={drawGrid} />
      </pixiContainer>
    </Application>
  );
}
