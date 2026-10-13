import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { SplashOverlay } from '../SplashOverlay';

vi.mock('../../api/client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://127.0.0.1:31337${path}`),
}));

describe('SplashOverlay', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en');
    vi.useFakeTimers();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true })),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('renders still art, fades after health succeeds, and unmounts after the fade', async () => {
    render(<SplashOverlay />);

    const overlay = screen.getByRole('status', { name: 'Loading...' });
    expect(overlay.querySelector('.dm-splash-image')).toHaveAttribute(
      'src',
      expect.stringContaining('splash'),
    );

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(overlay).toHaveClass('is-fading');

    act(() => {
      vi.advanceTimersByTime(320);
    });
    expect(screen.queryByRole('status', { name: 'Loading...' })).toBeNull();
  });

  it('a missing splash cannot hold healthy startup and the status is localized', async () => {
    await i18n.changeLanguage('ru');
    render(<SplashOverlay />);
    const overlay = screen.getByRole('status', { name: 'Загрузка...' });
    const image = overlay.querySelector('img');
    if (!image) throw Error('splash image missing');
    act(() => {
      image.dispatchEvent(new Event('error'));
    });
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    act(() => {
      vi.advanceTimersByTime(320);
    });
    expect(screen.queryByRole('status', { name: 'Загрузка...' })).toBeNull();
  });
});
