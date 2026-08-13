import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

vi.mock('../../api/campaigns', () => ({
  fetchCampaigns: vi.fn(async () => []),
  fetchCampaign: vi.fn(),
  createCampaign: vi.fn(),
}));

beforeEach(async () => {
  vi.clearAllMocks();
  useStore.setState(useStore.getInitialState());
  useStore.setState((state) => ({
    onboarding: { ...state.onboarding, completed: true },
    settings: { ...state.settings, uiLanguage: 'ru' },
  }));
  await i18n.changeLanguage('ru');
});

describe('production application root', () => {
  it('renders the Russian campaign home as the real production root', async () => {
    const { container } = render(<App />);

    expect(await screen.findByText('DUNGEON MASTER AI')).toBeInTheDocument();
    expect(screen.getByTestId('brand-crest').querySelector('svg')).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Ваши хроники' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Новая кампания' })).toBeVisible();
    expect(screen.queryByText(/создать комнату|join room/i)).not.toBeInTheDocument();
    expect(screen.queryByPlaceholderText('Что ты делаешь?')).not.toBeInTheDocument();
    expect(container.querySelector('.dm-vtt-panel')).not.toBeInTheDocument();
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

  it('switches the campaign home to English without changing roots', async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole('button', { name: 'EN' }));
    expect(await screen.findByRole('heading', { name: 'Your chronicles' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'New campaign' })).toBeVisible();
  });
});
