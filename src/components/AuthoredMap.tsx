import { useTranslation } from 'react-i18next';
import type { BattleMap } from '../api/contracts/battleMap';
export function AuthoredMap({ map, cellSize }: { map: BattleMap; cellSize: number }) {
  const { t } = useTranslation('combat');
  return (
    <svg
      data-testid="authored-map"
      data-width-cells={map.width}
      data-height-cells={map.height}
      width={map.width * cellSize}
      height={map.height * cellSize}
      style={{ position: 'absolute', left: 0, top: 0, pointerEvents: 'none' }}
      role="img"
      aria-label={t('authored.map_label')}
    >
      <title>{t('authored.map_label')}</title>
      <rect width="100%" height="100%" fill="var(--color-bg-base)" stroke="var(--color-accent)" />
      {map.blocked.map((cell) => (
        <rect
          data-testid="authored-wall"
          key={`${cell.x}:${cell.y}`}
          x={cell.x * cellSize}
          y={cell.y * cellSize}
          width={cellSize}
          height={cellSize}
          fill="var(--color-bg-raised)"
          stroke="var(--color-border-subtle)"
        />
      ))}
      {map.exits.map((cell) => (
        <g
          key={`${cell.x}:${cell.y}`}
          data-testid="authored-exit"
          data-cell-x={cell.x}
          data-cell-y={cell.y}
        >
          <rect
            x={cell.x * cellSize}
            y={cell.y * cellSize}
            width={cellSize}
            height={cellSize}
            fill="var(--color-bg-base)"
            stroke="var(--color-accent)"
            strokeWidth={2}
          />
          <text
            x={(cell.x + 0.5) * cellSize}
            y={(cell.y + 0.5) * cellSize}
            textAnchor="middle"
            dominantBaseline="middle"
            fill="var(--color-fg-primary)"
            fontSize={12}
            fontFamily="var(--font-body)"
          >
            {t('authored.exit')}
          </text>
        </g>
      ))}
    </svg>
  );
}
