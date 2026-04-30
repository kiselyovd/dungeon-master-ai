export type AoeShape = 'cone' | 'sphere' | 'line' | 'cube';

interface Props {
  shape: AoeShape;
  /** Origin in screen pixels (center of casting cell). */
  originX: number;
  originY: number;
  /** Cell size in pixels. */
  cellSize: number;
  /** Template size in feet (15 for cone, 20 for sphere radius, 30 for line, 15 for cube). */
  sizeInFt: number;
  /** Rotation in degrees (for cone and line). Defaults to 0 (pointing right). */
  rotateDeg?: number;
}

const FT_PER_CELL = 5;

/**
 * SVG-based AoE template overlay. Positioned absolutely within the VTT canvas.
 *
 * Colors are drawn from the school-of-magic palette in theme.css (already shipped):
 * - cone: var(--magic-evocation) (fire / force spells)
 * - sphere: var(--magic-conjuration)
 * - line: var(--magic-transmutation)
 * - cube: var(--magic-abjuration) (shielding area)
 */
export function AoeTemplate({ shape, originX, originY, cellSize, sizeInFt, rotateDeg = 0 }: Props) {
  const px = (ft: number) => (ft / FT_PER_CELL) * cellSize;

  switch (shape) {
    case 'cone': {
      const length = px(sizeInFt);
      const halfAngle = 30; // 60-degree cone
      const rad = (halfAngle * Math.PI) / 180;
      const dx1 = length * Math.cos(rad);
      const dy1 = length * Math.sin(rad);
      const points = `0,0 ${dx1},${-dy1} ${dx1},${dy1}`;
      return (
        <svg
          data-testid="aoe-cone"
          className="aoe-cone"
          style={{
            position: 'absolute',
            left: originX,
            top: originY,
            overflow: 'visible',
            transform: `rotate(${rotateDeg}deg)`,
            transformOrigin: '0 0',
            pointerEvents: 'none',
          }}
          width={length + 10}
          height={length + 10}
          aria-hidden="true"
        >
          <polygon
            points={points}
            fill="var(--magic-evocation)"
            fillOpacity={0.3}
            stroke="var(--magic-evocation)"
            strokeOpacity={0.6}
            strokeWidth={1}
          />
        </svg>
      );
    }
    case 'sphere': {
      const radius = px(sizeInFt);
      return (
        <svg
          data-testid="aoe-sphere"
          className="aoe-sphere"
          style={{
            position: 'absolute',
            left: originX - radius,
            top: originY - radius,
            overflow: 'visible',
            pointerEvents: 'none',
          }}
          width={radius * 2}
          height={radius * 2}
          aria-hidden="true"
        >
          <circle
            cx={radius}
            cy={radius}
            r={radius}
            fill="var(--magic-conjuration)"
            fillOpacity={0.3}
            stroke="var(--magic-conjuration)"
            strokeOpacity={0.6}
            strokeWidth={1}
          />
        </svg>
      );
    }
    case 'line': {
      const length = px(sizeInFt);
      const width = px(5); // 5ft wide per 5e RAW
      return (
        <svg
          data-testid="aoe-line"
          className="aoe-line"
          style={{
            position: 'absolute',
            left: originX,
            top: originY - width / 2,
            overflow: 'visible',
            transform: `rotate(${rotateDeg}deg)`,
            transformOrigin: '0 50%',
            pointerEvents: 'none',
          }}
          width={length}
          height={width}
          aria-hidden="true"
        >
          <rect
            x={0}
            y={0}
            width={length}
            height={width}
            fill="var(--magic-transmutation)"
            fillOpacity={0.3}
            stroke="var(--magic-transmutation)"
            strokeOpacity={0.6}
            strokeWidth={1}
          />
        </svg>
      );
    }
    case 'cube': {
      const size = px(sizeInFt);
      return (
        <svg
          data-testid="aoe-cube"
          className="aoe-cube"
          style={{
            position: 'absolute',
            left: originX,
            top: originY,
            overflow: 'visible',
            pointerEvents: 'none',
          }}
          width={size}
          height={size}
          aria-hidden="true"
        >
          <rect
            x={0}
            y={0}
            width={size}
            height={size}
            fill="var(--magic-abjuration)"
            fillOpacity={0.3}
            stroke="var(--magic-abjuration)"
            strokeOpacity={0.6}
            strokeWidth={1}
          />
        </svg>
      );
    }
  }
}
