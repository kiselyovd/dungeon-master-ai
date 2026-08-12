import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { useStore } from '../../state/useStore';
import App from '../App';

const windowActions = vi.hoisted(() => ({
  close: vi.fn(async () => {}),
  minimize: vi.fn(async () => {}),
  toggleMaximize: vi.fn(async () => {}),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    close: windowActions.close,
    isFullscreen: vi.fn(async () => false),
    minimize: windowActions.minimize,
    setFullscreen: vi.fn(async () => {}),
    toggleMaximize: windowActions.toggleMaximize,
  }),
}));

beforeEach(async () => {
  vi.clearAllMocks();
  useStore.setState(useStore.getInitialState());
  useStore.setState((state) => ({
    onboarding: { ...state.onboarding, completed: true },
    settings: { ...state.settings, uiLanguage: 'en' },
  }));
  await i18n.changeLanguage('en');
});

describe('production application root', () => {
  it('renders the adapted product shell and default campaign identity', async () => {
    const { container } = render(<App />);

    expect(await screen.findByText('DUNGEON MASTER AI')).toBeInTheDocument();
    expect(screen.getByTestId('brand-crest').querySelector('svg')).toBeInTheDocument();
    expect(screen.getByText('Untitled Campaign')).toBeInTheDocument();
    expect(screen.getByText('The Adventure')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('What do you do?')).toBeInTheDocument();
    expect(container.querySelector('.dm-vtt-empty-art')).toHaveAttribute(
      'src',
      expect.stringContaining('vtt-empty'),
    );
    expect(container.querySelector('[data-art-direction="living-tabletop"]')).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText(/Anthropic|Claude/i)).not.toBeInTheDocument());
  });

  it('keeps native window controls operable while onboarding is open', async () => {
    useStore.setState((state) => ({
      onboarding: { ...state.onboarding, completed: false },
    }));

    render(<App />);

    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    screen.getByTestId('window-minimize').click();
    screen.getByTestId('window-maximize').click();
    screen.getByTestId('window-close').click();

    await waitFor(() => {
      expect(windowActions.minimize).toHaveBeenCalledOnce();
      expect(windowActions.toggleMaximize).toHaveBeenCalledOnce();
      expect(windowActions.close).toHaveBeenCalledOnce();
    });
  });

  it('renders the same production root with Russian visible labels', async () => {
    useStore.setState((state) => ({
      settings: { ...state.settings, uiLanguage: 'ru' },
    }));
    await i18n.changeLanguage('ru');

    render(<App />);

    expect(await screen.findByText('DUNGEON MASTER AI')).toBeInTheDocument();
    expect(screen.getByText('Без названия')).toBeInTheDocument();
    expect(screen.getByText('Приключение')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Что ты делаешь?')).toBeInTheDocument();
  });
});
