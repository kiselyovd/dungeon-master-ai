import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
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

vi.mock('../../api/rooms', async () => {
  const { default: wire } = await import('../../api/__fixtures__/roomUpdate.json');
  const member = wire.state.room.members.find((member) => member.role === 'player');
  return {
    recoverHostRoom: vi.fn(async () => null),
    recoverGuestRoom: vi.fn(async () => null),
    savedGuestRooms: vi.fn(async () => []),
    postponeGuestRoom: vi.fn(async () => {}),
    resumeGuestRoom: vi.fn(),
    hostRoom: vi.fn(),
    stopHostRoom: vi.fn(async () => {}),
    joinRoom: vi.fn(async () => ({
      kind: 'guest',
      room: wire.state.room,
      member_id: member?.member_id,
      address: '127.0.0.1:4123',
      join_id: '55555555-5555-4555-8555-555555555555',
    })),
    loadRoom: vi.fn(async () => wire),
    loadPendingRoomAction: vi.fn(async () => null),
    sendRoomAction: vi.fn(async () => {}),
  };
});
vi.mock('../../api/roomStream', () => ({ streamRoom: vi.fn(() => new Promise<void>(() => {})) }));

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
  afterEach(() => vi.restoreAllMocks());

  it('does not mount state-writing game views before native persistence hydration finishes', async () => {
    let finish = () => {};
    vi.spyOn(useStore.persist, 'hasHydrated').mockReturnValue(false);
    vi.spyOn(useStore.persist, 'onFinishHydration').mockImplementation((callback) => {
      finish = () => callback(useStore.getState());
      return () => {};
    });
    const loadError = vi.spyOn(useStore.getState().session, 'setLoadError');
    const { container } = render(<App />);
    expect(container.querySelector('.dm-chat-panel')).toBeNull();
    expect(loadError).not.toHaveBeenCalled();
    await act(async () => finish());
    expect(await screen.findByRole('heading', { name: 'Ваши хроники' })).toBeVisible();
  });
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

it('opens co-op through the real root and displays the human-owned guest hero', async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByRole('button', { name: 'Совместная игра' }));
  expect(screen.getByRole('heading', { name: 'Совместная игра' })).toBeVisible();
  await user.type(screen.getByLabelText('Адрес и порт организатора'), '127.0.0.1:4123');
  await user.type(screen.getByLabelText('Код комнаты'), '11111111-1111-4111-8111-111111111111');
  await user.type(screen.getByLabelText('Ваше имя игрока'), 'Tara');
  await user.type(screen.getByLabelText('Приглашение'), 'fixture-invitation');
  await user.click(screen.getByRole('button', { name: 'Подключиться' }));
  expect(await screen.findByRole('heading', { name: 'Мастер игры · ИИ' })).toBeVisible();
  expect(screen.getByRole('heading', { name: 'Герои игроков' })).toBeVisible();
  expect(screen.getAllByText('Tara', { exact: false }).length).toBeGreaterThan(0);
  expect(screen.getByLabelText('Что делает ваш герой?')).toBeEnabled();
  await user.keyboard('{Control>}{Shift>}S{/Shift}{/Control}');
  expect(useStore.getState().saves.isOpen).toBe(false);
  expect(screen.queryByRole('button', { name: 'Создать своего героя' })).not.toBeInTheDocument();
  expect(useStore.getState().pc.heroClass).toBeNull();
});

it('lets a first-boot guest join without configuring a provider or completing solo onboarding', async () => {
  const user = userEvent.setup();
  useStore.setState((state) => ({ onboarding: { ...state.onboarding, completed: false } }));
  const before = structuredClone(useStore.getState().settings.providers);
  render(<App />);
  await user.click(await screen.findByRole('button', { name: 'Подключиться к игре' }));
  expect(screen.getByRole('heading', { name: 'Совместная игра' })).toBeVisible();
  expect(useStore.getState().onboarding.completed).toBe(false);
  await user.click(screen.getByRole('button', { name: 'Назад к кампаниям' }));
  expect(await screen.findByRole('dialog')).toBeVisible();
  expect(useStore.getState().settings.providers).toEqual(before);
});
