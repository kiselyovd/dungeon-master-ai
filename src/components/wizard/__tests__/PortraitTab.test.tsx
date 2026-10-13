import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { PortraitTab } from '../PortraitTab';

vi.mock('../../../api/client', () => ({
  backendUrl: vi.fn().mockResolvedValue('http://test/image/generate'),
}));

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('PortraitTab', () => {
  it('uses the backend bundled portrait identity and never stores its data URL', async () => {
    useStore.getState().settings.setImageEnabled(true);
    useStore.getState().settings.setImagePreset('balanced');
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(
            JSON.stringify({
              source: 'bundled',
              asset_id: 'portrait-fighter-01-v2',
              url: 'data:image/webp;base64,not-persisted',
            }),
            { status: 200 },
          ),
      ),
    );
    render(<PortraitTab />);
    fireEvent.click(screen.getByRole('button', { name: /^Generate$/i }));
    await screen.findByAltText(/Character portrait/i);
    expect(useStore.getState().charCreation.portraitAssetId).toBe('hero.fighter.01.v2');
    expect(useStore.getState().charCreation.portraitUrl).toBeNull();
    vi.unstubAllGlobals();
  });
  it('selects an offline built-in identity without persisting image bytes or changing the hero name', () => {
    useStore.getState().settings.setImageEnabled(false);
    useStore.getState().charCreation.setDraftField('classId', 'fighter');
    useStore.getState().charCreation.setDraftField('name', 'My existing hero');
    render(<PortraitTab />);
    const choice = screen.getByRole('radio', { name: 'Look 2' });
    fireEvent.click(choice);
    const draft = useStore.getState().charCreation;
    expect(draft.portraitAssetId).toBe('hero.fighter.01.v2');
    expect(draft.portraitUrl).toBeNull();
    expect(draft.name).toBe('My existing hero');
    useStore.getState().settings.setImageEnabled(true);
  });
  it('renders empty state with prompt builder', () => {
    render(<PortraitTab />);
    expect(screen.getByText(/no portrait yet/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/prompt/i)).toBeInTheDocument();
  });

  it.each(
    ['fighter', 'rogue', 'wizard', 'cleric'].flatMap((heroClass) =>
      ['01', '02', '03', '04'].map((number) => [heroClass, number]),
    ),
  )('%s %s changes only cosmetic draft fields', (heroClass, number) => {
    const draft = useStore.getState().charCreation;
    draft.setDraftField('classId', heroClass ?? null);
    draft.setDraftField('name', 'Existing human hero');
    draft.setDraftField('raceId', 'human');
    draft.setAbilityScore('str', 16);
    draft.setDraftField('equipmentInventory', [
      { id: 'sword', name: 'Longsword', count: 1, icon: 'sword' },
    ]);
    const before = useStore.getState().charCreation;
    const { container } = render(<PortraitTab />);
    const choice = container.querySelector<HTMLInputElement>(
      `[data-asset-id="hero.${heroClass}.${number}.v2"] input`,
    );
    expect(choice).not.toBeNull();
    if (!choice) throw new Error('missing fighter identity');
    fireEvent.click(choice);
    const after = useStore.getState().charCreation;
    expect(after.portraitAssetId).toBe(`hero.${heroClass}.${number}.v2`);
    for (const key of Object.keys(before) as Array<keyof typeof before>) {
      if (['portraitAssetId', 'portraitMediaRef', 'portraitUrl'].includes(key)) continue;
      expect(after[key], key).toEqual(before[key]);
    }
  });

  it('seeds the prompt from race + class + alignment', () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('raceId', 'human');
    s.setDraftField('classId', 'fighter');
    s.setDraftField('alignment', 'LG');
    render(<PortraitTab />);
    const textarea = screen.getByLabelText(/prompt/i) as HTMLTextAreaElement;
    expect(textarea.value).toMatch(/human/);
    expect(textarea.value).toMatch(/fighter/);
    expect(textarea.value).toMatch(/LG/);
  });

  it('Skip clears portraitUrl and portraitPrompt', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('portraitUrl', 'http://example/cached.png');
    s.setDraftField('portraitPrompt', 'old prompt');
    render(<PortraitTab />);
    const skipBtn = screen.getByRole('button', { name: /skip/i });
    skipBtn.click();
    expect(useStore.getState().charCreation.portraitUrl).toBeNull();
    expect(useStore.getState().charCreation.portraitPrompt).toBeNull();
  });
});

describe('PortraitTab - graceful degradation', () => {
  const originalImageEnabled = useStore.getState().settings.imageEnabled;
  const originalImagePreset = useStore.getState().settings.imagePreset;
  const originalReplicateApiKey = useStore.getState().settings.replicateApiKey;

  afterEach(() => {
    useStore.getState().settings.setImageEnabled(originalImageEnabled);
    useStore.getState().settings.setImagePreset(originalImagePreset);
    useStore.getState().settings.setReplicateApiKey(originalReplicateApiKey);
  });

  it('shows disabled card with Skip button when imageEnabled is false', () => {
    useStore.getState().settings.setImageEnabled(false);
    render(<PortraitTab />);
    expect(screen.getByTestId('portrait-image-disabled-card')).toBeInTheDocument();
    const skipBtn = screen.getByRole('button', { name: /skip/i });
    skipBtn.click();
    expect(useStore.getState().charCreation.portraitUrl).toBeNull();
    expect(useStore.getState().charCreation.portraitPrompt).toBeNull();
  });

  it('shows no-provider card when cloud preset is selected but replicateApiKey is null', () => {
    useStore.getState().settings.setImageEnabled(true);
    useStore.getState().settings.setImagePreset('cloud');
    useStore.getState().settings.setReplicateApiKey(null);
    render(<PortraitTab />);
    expect(screen.getByTestId('portrait-image-no-provider-card')).toBeInTheDocument();
  });

  it('calls onOpenSettings when Enable button is clicked on disabled card', () => {
    useStore.getState().settings.setImageEnabled(false);
    const handleOpenSettings = vi.fn();
    render(<PortraitTab onOpenSettings={handleOpenSettings} />);
    const enableBtn = screen.getByTestId('portrait-enable-btn');
    enableBtn.click();
    expect(handleOpenSettings).toHaveBeenCalledOnce();
  });

  it('calls onOpenSettings when Configure button is clicked on no-provider card', () => {
    useStore.getState().settings.setImageEnabled(true);
    useStore.getState().settings.setImagePreset('cloud');
    useStore.getState().settings.setReplicateApiKey(null);
    const handleOpenSettings = vi.fn();
    render(<PortraitTab onOpenSettings={handleOpenSettings} />);
    const configureBtn = screen.getByTestId('portrait-configure-btn');
    configureBtn.click();
    expect(handleOpenSettings).toHaveBeenCalledOnce();
  });
});
