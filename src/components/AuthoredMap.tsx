import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { BattleMap } from '../api/contracts/battleMap';
import { calibratedMapSkin } from '../lib/mapSkins';
export function AuthoredMap({
  map,
  cellSize,
  skinAssetId,
  showSkin = true,
}: {
  map: BattleMap;
  cellSize: number;
  skinAssetId?: string | null;
  showSkin?: boolean;
}) {
  const { t } = useTranslation('combat');
  const skin = calibratedMapSkin(skinAssetId, map);
  const [failedUri, setFailedUri] = useState<string | null>(null);
  const visible = showSkin && Boolean(skin) && failedUri !== skin?.uri;
  useEffect(() => {
    if (skinAssetId && !skin) console.warn('[map:WARN] skin_calibration_rejected');
    else if (skin) console.info('[map:INFO] calibrated_skin_bound', { assetId: skin.id });
  }, [skinAssetId, skin]);
  return (
    <svg
      data-testid="authored-map"
      data-width-cells={map.width}
      data-height-cells={map.height}
      data-skin-state={
        !skin ? 'neutral' : failedUri === skin.uri ? 'unavailable' : showSkin ? 'visible' : 'hidden'
      }
      width={map.width * cellSize}
      height={map.height * cellSize}
      style={{ position: 'absolute', left: 0, top: 0, pointerEvents: 'none' }}
      role="img"
      aria-label={t('authored.map_label')}
    >
      <title>{t('authored.map_label')}</title>
      <rect width="100%" height="100%" fill="var(--color-bg-base)" stroke="var(--color-accent)" />
      {visible && skin && (
        <image
          data-testid="authored-map-skin"
          data-asset-id={skin.id}
          href={skin.uri}
          x={0}
          y={0}
          width={map.width * cellSize}
          height={map.height * cellSize}
          preserveAspectRatio="none"
          onError={() => {
            setFailedUri(skin.uri);
            console.warn('[map:WARN] skin_resource_unavailable');
          }}
        />
      )}
      {map.blocked.map((cell) => (
        <rect
          data-testid="authored-wall"
          key={`${cell.x}:${cell.y}`}
          data-cell-x={cell.x}
          data-cell-y={cell.y}
          x={cell.x * cellSize}
          y={cell.y * cellSize}
          width={cellSize}
          height={cellSize}
          fill={visible ? 'transparent' : 'var(--color-bg-raised)'}
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
            fill={
              visible
                ? 'color-mix(in srgb, var(--color-bg-base) 75%, transparent)'
                : 'var(--color-bg-base)'
            }
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
