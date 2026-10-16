import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { DecorativeImage } from '../DecorativeImage';

it('failed atmosphere keeps the owning action usable and a new resource can recover', () => {
  const view = render(
    <div>
      <DecorativeImage src="first.webp" assetId="splash.v2" data-testid="art" />
      <button type="button">Continue</button>
    </div>,
  );
  fireEvent.error(screen.getByTestId('art'));
  expect(screen.getByTestId('art')).toHaveStyle({ visibility: 'hidden' });
  expect(screen.getByRole('button', { name: 'Continue' })).toBeVisible();
  view.rerender(
    <div>
      <DecorativeImage src="second.webp" assetId="onboarding.v2" data-testid="art" />
      <button type="button">Continue</button>
    </div>,
  );
  expect(screen.getByTestId('art')).not.toHaveStyle({ visibility: 'hidden' });
  fireEvent.load(screen.getByTestId('art'));
  expect(screen.getByTestId('art')).toHaveAttribute('alt', '');
});
