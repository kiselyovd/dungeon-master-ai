import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, it } from 'vitest';
import i18n from '../../i18n';
import { SceneArtwork, sceneArtworkAsset } from '../SceneArtwork';

beforeEach(async () => {
  await i18n.changeLanguage('en');
});
it('opens only the current full painting, closes stale previews and never reopens on return', () => {
  const town = { assetId: 'scene.bell.town.v2', location: 'town' as const };
  const view = render(<SceneArtwork presentation={town} stage />);
  fireEvent.click(screen.getByRole('button', { name: 'Open painting: Veresk' }));
  expect(view.container.contains(screen.getByRole('dialog'))).toBe(false);
  expect(screen.getByTestId('scene-art-preview')).toHaveAttribute('data-location', 'town');
  view.rerender(
    <SceneArtwork presentation={{ assetId: 'scene.bell.quay.v2', location: 'quay' }} stage />,
  );
  expect(screen.queryByRole('dialog')).toBeNull();
  view.rerender(<SceneArtwork presentation={town} stage />);
  expect(screen.queryByRole('dialog')).toBeNull();
});
it('wrong-location or unknown art stays neutral without revealing another scene', () => {
  expect(sceneArtworkAsset({ assetId: 'scene.bell.belfry.v2', location: 'town' })).toBeNull();
  const view = render(
    <SceneArtwork presentation={{ assetId: 'scene.bell.town.v999', location: 'town' }} stage />,
  );
  expect(screen.getByText('Painting unavailable')).toBeVisible();
  expect(view.container.querySelector('img')).toBeNull();
  expect(screen.getByRole('button', { name: 'Open painting: Veresk' })).toBeDisabled();
});
it('failed network art retains the current title and a later scene can load', () => {
  const view = render(
    <SceneArtwork presentation={{ assetId: 'scene.bell.town.v2', location: 'town' }} stage />,
  );
  const image = view.container.querySelector('img');
  if (!image) throw Error('missing art');
  fireEvent.error(image);
  expect(screen.getByText('Painting unavailable')).toBeVisible();
  view.rerender(
    <SceneArtwork presentation={{ assetId: 'scene.bell.quay.v2', location: 'quay' }} stage />,
  );
  expect(screen.queryByText('Painting unavailable')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Old quay' })).toBeVisible();
});
