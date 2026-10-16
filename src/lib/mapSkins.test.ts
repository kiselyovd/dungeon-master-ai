import { expect, it } from 'vitest';
import type { BattleMap } from '../api/contracts/battleMap';
import { calibratedMapSkin } from './mapSkins';

const archive: BattleMap = {
  width: 12,
  height: 8,
  blocked: [0, 1, 2, 4, 5, 6, 7].map((y) => ({ x: 6, y })),
  exits: [{ x: 1, y: 3 }],
};
it('binds only the explicit calibrated ID to exact geometry, independent of array order', () => {
  expect(calibratedMapSkin('map.bell_archive.v2', archive)?.id).toBe('map.bell_archive.v2');
  expect(
    calibratedMapSkin('map.bell_archive.v2', {
      ...archive,
      blocked: [...archive.blocked].reverse(),
    }),
  ).not.toBeNull();
  expect(calibratedMapSkin(null, archive)).toBeNull();
  expect(calibratedMapSkin('unknown.future', archive)).toBeNull();
  for (const changed of [
    { ...archive, width: 13 },
    { ...archive, height: 9 },
    { ...archive, blocked: [...archive.blocked, { x: 6, y: 3 }] },
    { ...archive, blocked: archive.blocked.slice(1) },
    { ...archive, blocked: [...archive.blocked, { x: 6, y: 0 }] },
    { ...archive, exits: [{ x: 2, y: 3 }] },
  ])
    expect(calibratedMapSkin('map.bell_archive.v2', changed)).toBeNull();
});
