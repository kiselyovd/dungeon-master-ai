import type { BattleMap } from '../api/contracts/battleMap';
import { visualAsset } from '../assets/visualCatalog';

const cells = (values: BattleMap['blocked']) =>
  values
    .map((cell) => `${cell.x},${cell.y}`)
    .sort()
    .join(';');
export const ARCHIVE_CALIBRATION = {
  assetId: 'map.bell_archive.v2',
  width: 12,
  height: 8,
  blocked: '6,0;6,1;6,2;6,4;6,5;6,6;6,7',
  exits: '1,3',
  sourceFrame: { x: 0, y: 0, width: 1536, height: 1024 },
} as const;
export function calibratedMapSkin(assetId: string | null | undefined, map: BattleMap) {
  const reference = ARCHIVE_CALIBRATION;
  if (
    assetId !== reference.assetId ||
    map.width !== reference.width ||
    map.height !== reference.height ||
    cells(map.blocked) !== reference.blocked ||
    cells(map.exits) !== reference.exits
  )
    return null;
  const asset = visualAsset(assetId, 'map');
  return asset &&
    asset.width === reference.sourceFrame.width &&
    asset.height === reference.sourceFrame.height
    ? asset
    : null;
}
