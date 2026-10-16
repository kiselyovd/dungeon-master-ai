import '../../i18n';
import { fireEvent, render } from '@testing-library/react';
import { expect, it } from 'vitest';
import type { BattleMap } from '../../api/contracts/battleMap';
import { AuthoredMap } from '../AuthoredMap';

const map: BattleMap = {
  width: 12,
  height: 8,
  blocked: [0, 1, 2, 4, 5, 6, 7].map((y) => ({ x: 6, y })),
  exits: [{ x: 1, y: 3 }],
};
it.each([
  30, 48, 64,
])('skin shares geometry origin/frame at cell size%s; hiding/failure never changes wall or exit overlays', (cellSize) => {
  const view = render(
    <AuthoredMap map={map} cellSize={cellSize} skinAssetId="map.bell_archive.v2" />,
  );
  const image = view.getByTestId('authored-map-skin');
  expect(image).toHaveAttribute('width', String(12 * cellSize));
  expect(image).toHaveAttribute('height', String(8 * cellSize));
  const walls = view.getAllByTestId('authored-wall');
  expect(walls).toHaveLength(7);
  expect(walls.every((wall) => wall.getAttribute('x') === String(6 * cellSize))).toBe(true);
  expect(view.getByTestId('authored-exit')).toHaveAttribute('data-cell-x', '1');
  view.rerender(
    <AuthoredMap
      map={map}
      cellSize={cellSize}
      skinAssetId="map.bell_archive.v2"
      showSkin={false}
    />,
  );
  expect(view.queryByTestId('authored-map-skin')).toBeNull();
  expect(view.getAllByTestId('authored-wall')).toHaveLength(7);
  view.rerender(<AuthoredMap map={map} cellSize={cellSize} skinAssetId="map.bell_archive.v2" />);
  fireEvent.error(view.getByTestId('authored-map-skin'));
  expect(view.queryByTestId('authored-map-skin')).toBeNull();
  expect(view.getAllByTestId('authored-wall')).toHaveLength(7);
  expect(view.getByTestId('authored-map')).toHaveAttribute('data-skin-state', 'unavailable');
});
it('a changed passage falls back to the supplied vector geometry', () => {
  const changed = { ...map, blocked: map.blocked.slice(1) };
  const view = render(
    <AuthoredMap map={changed} cellSize={30} skinAssetId="map.bell_archive.v2" />,
  );
  expect(view.queryByTestId('authored-map-skin')).toBeNull();
  expect(view.getAllByTestId('authored-wall')).toHaveLength(6);
});
