import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getPendingAgentRecoveryInput } from '../../api/agentPending';
import { fetchAgentRecovery } from '../../api/agentRecovery';
import { fetchCampaigns } from '../../api/campaigns';
import i18n from '../../i18n';
import { parseApiKey, parseBaseUrl } from '../../state/providers';
import { useStore } from '../../state/useStore';
import App from '../App';

const windowActions = vi.hoisted(() => ({
  close: vi.fn(async () => {}),
  minimize: vi.fn(async () => {}),
  toggleMaximize: vi.fn(async () => {}),
}));
vi.mock('../../api/agentRecovery', () => ({ fetchAgentRecovery: vi.fn(async () => null) }));
vi.mock('../../api/agentPending', () => ({
  getPendingAgentRecoveryInput: vi.fn(async () => null),
  getPendingAgentClarification: vi.fn(async () => null),
}));
vi.mock('../../api/chat', () => ({ fetchSessionMessages: vi.fn(async () => []) }));
vi.mock('../../components/VttCanvas', () => ({
  VttCanvas: () => <div data-testid="production-vtt" />,
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
  vi.mocked(fetchAgentRecovery).mockResolvedValue(null);
  vi.mocked(getPendingAgentRecoveryInput).mockResolvedValue(null);
  useStore.setState(useStore.getInitialState());
  useStore.setState((state) => ({
    onboarding: { ...state.onboarding, completed: true },
    settings: { ...state.settings, uiLanguage: 'ru' },
  }));
  await i18n.changeLanguage('ru');
});

describe('production application root', () => {
  afterEach(() => vi.restoreAllMocks());
  it('keeps local chat setup gated when the process runs but provider configuration is unconfirmed', async () => {
    vi.spyOn(useStore.persist, 'hasHydrated').mockReturnValue(true);
    vi.mocked(fetchAgentRecovery).mockResolvedValue(null);
    useStore.getState().pc.applyPreset('fighter');
    useStore
      .getState()
      .session.activate(
        '11111111-1111-4111-8111-111111111111',
        '22222222-2222-4222-8222-222222222222',
      );
    useStore.getState().session.setCurrentScene({ name: 'First scene', stepCounter: 0 });
    useStore.getState().campaigns.setActive({
      id: '11111111-1111-4111-8111-111111111111',
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Campaign',
      premise: 'Opening',
      revision: 1,
      last_session_id: '22222222-2222-4222-8222-222222222222',
      created_at: '2026-10-12T00:00:00Z',
      updated_at: '2026-10-12T00:00:00Z',
      seed: { kind: 'collaborative', answers: {} },
    });
    useStore.setState((state) => ({
      pc: { ...state.pc, characterId: '33333333-3333-4333-8333-333333333333' },
      settings: {
        ...state.settings,
        activeProvider: 'local-mistralrs',
        providers: { ...state.settings.providers, 'local-mistralrs': null },
      },
    }));
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          llm: { state: 'ready', port: 34567 },
          image: { state: 'off' },
          model_id: 'gemma4_e2b',
        }),
        { status: 200 },
      ),
    );
    render(<App />);
    expect(await screen.findByRole('button', { name: 'Настроить мастера' })).toBeVisible();
    const surface = document.querySelector('[data-game-table]');
    expect(surface).toHaveAttribute('data-table-mode', 'solo');
    expect(surface).toHaveAttribute('data-table-readiness', 'needs_model');
  });
  it('shows confirmed recovery before a new turn and preserves an unsent draft', async () => {
    const campaignId = '11111111-1111-4111-8111-111111111111';
    const sessionId = '22222222-2222-4222-8222-222222222222';
    const actorId = '33333333-3333-4333-8333-333333333333';
    const requestId = '44444444-4444-4444-8444-444444444444';
    vi.spyOn(useStore.persist, 'hasHydrated').mockReturnValue(true);
    useStore.getState().pc.applyPreset('fighter');
    useStore.getState().session.activate(campaignId, sessionId);
    useStore.getState().session.setCurrentScene({ name: 'First scene', stepCounter: 0 });
    const baseUrl = parseBaseUrl('http://localhost:31415/v1');
    const apiKey = parseApiKey('fixture-value');
    if (!baseUrl || !apiKey) throw new Error('valid fixture provider required');
    useStore.setState((state) => ({
      pc: { ...state.pc, characterId: actorId },
      settings: {
        ...state.settings,
        activeProvider: 'openai-compat',
        providers: {
          ...state.settings.providers,
          'openai-compat': {
            kind: 'openai-compat',
            baseUrl,
            model: 'fixture',
            apiKey,
          },
        },
      },
    }));
    useStore.getState().campaigns.setActive({
      id: campaignId,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Хроника',
      premise: 'История',
      revision: 1,
      last_session_id: sessionId,
      created_at: '2026-10-12T00:00:00Z',
      updated_at: '2026-10-12T00:00:00Z',
      seed: { kind: 'collaborative', answers: {} },
    });
    vi.mocked(fetchAgentRecovery).mockResolvedValue({
      schema_version: 1,
      scope: {
        request_id: requestId,
        campaign_id: campaignId,
        session_id: sessionId,
        actor_id: actorId,
        authority: { kind: 'solo', restore_generation: 0 },
      },
      model: 'fixture',
      phase: 'narration_pending',
      allowed_action: 'resume',
      progress_known: true,
      commands: [{ command_id: 1, state: 'finished' }],
      confirmed_effects: [],
      clarification: null,
    });
    vi.mocked(getPendingAgentRecoveryInput).mockResolvedValue({
      input: { requestId, campaignId, sessionId, playerMessage: 'Исходное действие', history: [] },
    });
    render(<App />);
    await screen.findByRole('button', { name: 'Продолжить ответ' });
    expect(screen.getByText('Завершено шагов: 1 из 1')).toBeInTheDocument();
    const draft = screen.getByRole('textbox', { name: 'Что ты делаешь?' });
    await userEvent.type(draft, 'Новый черновик');
    expect(screen.getByRole('button', { name: 'Отправить' })).toBeDisabled();
    expect(draft).toHaveValue('Новый черновик');
  });

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
    expect(await screen.findByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
  });
  it('renders the Russian campaign home as the real production root', async () => {
    const { container } = render(<App />);

    expect(await screen.findByText('DUNGEON MASTER AI')).toBeInTheDocument();
    expect(screen.getByTestId('brand-crest').querySelector('svg')).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
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
    expect(await screen.findByRole('heading', { name: 'Your adventures' })).toBeVisible();
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
  expect(screen.getByRole('group', { name: 'Герои игроков' })).toBeVisible();
  expect(screen.getAllByText('Tara', { exact: false }).length).toBeGreaterThan(0);
  expect(screen.getByLabelText('Что делает ваш герой?')).toBeEnabled();
  await user.keyboard('{Control>}{Shift>}S{/Shift}{/Control}');
  expect(useStore.getState().saves.isOpen).toBe(false);
  expect(screen.queryByRole('button', { name: 'Создать своего героя' })).not.toBeInTheDocument();
  expect(useStore.getState().pc.heroClass).toBeNull();
  await user.keyboard('{Control>}k{/Control}');
  const palette = screen.getByRole('dialog', { name: 'Команды' });
  expect(palette.querySelector('[data-command-id="quick_save"]')).toBeDisabled();
  expect(palette.querySelector('[data-command-id="settings"]')).toBeDisabled();
  expect(palette.querySelector('[data-command-id="end_turn"]')).toBeDisabled();
  expect(palette.querySelector('[data-command-id="focus_map"]')).toBeDisabled();
});

it('lets a first-boot guest join without configuring a provider or creating a solo hero', async () => {
  const user = userEvent.setup();
  useStore.setState((state) => ({ onboarding: { ...state.onboarding, completed: false } }));
  const before = structuredClone(useStore.getState().settings.providers);
  render(<App />);
  await user.click(await screen.findByRole('button', { name: 'Присоединиться к игре' }));
  expect(screen.getByRole('heading', { name: 'Совместная игра' })).toBeVisible();
  expect(useStore.getState().onboarding).toMatchObject({ completed: true, entry: 'guest' });
  await user.click(screen.getByRole('button', { name: 'Назад к кампаниям' }));
  expect(await screen.findByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(useStore.getState().settings.providers).toEqual(before);
});

it('loads the host campaign catalog when entering co-op from a cold restored solo table', async () => {
  vi.spyOn(useStore.persist, 'hasHydrated').mockReturnValue(true);
  const id = crypto.randomUUID();
  const sessionId = crypto.randomUUID();
  const campaign = {
    id,
    schema_version: 1 as const,
    ruleset_id: 'srd-5.2.1' as const,
    title: 'Cold host campaign',
    premise: 'Opening',
    revision: 1,
    last_session_id: sessionId,
    created_at: '2026-10-13T00:00:00Z',
    updated_at: '2026-10-13T00:00:00Z',
    seed: { kind: 'collaborative' as const, answers: {} },
  };
  useStore.getState().pc.applyPreset('fighter');
  useStore.getState().session.activate(id, sessionId);
  useStore.getState().campaigns.setActive(campaign);
  useStore.setState((state) => ({ pc: { ...state.pc, characterId: crypto.randomUUID() } }));
  vi.mocked(fetchCampaigns).mockResolvedValueOnce([campaign]);
  render(<App />);
  expect(useStore.getState().campaigns.list).toEqual([]);
  await userEvent.click(await screen.findByRole('button', { name: 'Совместная игра' }));
  await waitFor(() => expect(fetchCampaigns).toHaveBeenCalled());
  await waitFor(() =>
    expect(screen.getByRole('option', { name: 'Cold host campaign' })).toHaveValue(id),
  );
  expect(screen.getByRole('button', { name: 'Создать комнату' })).toBeEnabled();
});
