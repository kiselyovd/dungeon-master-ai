import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
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
