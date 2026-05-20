import { render, screen } from '@testing-library/react';
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
  it('renders empty state with prompt builder', () => {
    render(<PortraitTab />);
    expect(screen.getByText(/no portrait yet/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/prompt/i)).toBeInTheDocument();
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
