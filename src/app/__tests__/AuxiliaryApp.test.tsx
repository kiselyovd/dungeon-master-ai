import { act, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { readFileSync } from 'node:fs';
import { emptyPcData } from '../../state/pc';
import { AuxiliaryApp } from '../AuxiliaryApp';

const native = vi.hoisted(() => ({ kind: 'hero', read: vi.fn(), close: vi.fn(), focus: vi.fn() }));
vi.mock('../../api/auxiliaryWindows', () => ({
  auxiliaryKind: () => native.kind,
  readAuxiliaryProjection: native.read,
  closeAuxiliaryWindow: native.close,
  focusMainWindow: native.focus,
}));
const actor = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const campaign = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const asset = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
const preview = `data:image/png;base64,${readFileSync('e2e/fixtures/sample.png').toString('base64')}`;
function portraitFrame() {
  const value = frame();
  return {
    ...value,
    hero: {
      ...value.hero,
      portraitMediaRef: {
        source: 'owned',
        asset_id: asset,
        category: 'portrait',
        origin: 'uploaded',
        descriptor: {
          sha256: '0'.repeat(64),
          mime_type: 'image/png',
          byte_len: 1,
          width: 1,
          height: 1,
        },
      },
    },
    portrait: { assetId: asset, dataUrl: preview },
    portraitLoading: false,
  };
}
function frame(scope = 'owned', hero = true) {
  return {
    schemaVersion: 1,
    sequence: 1,
    scopeId: scope,
    campaignId: campaign,
    actorId: actor,
    revision: 3,
    language: 'en',
    ready: true,
    hero: hero
      ? {
          ...emptyPcData(),
          characterId: actor,
          name: 'Mira',
          heroClass: 'fighter',
          hp: 9,
          hpMax: 12,
          backstory: 'My own biography',
        }
      : null,
    journal: [],
  };
}
describe('real detached production composition', () => {
  beforeEach(() => {
    vi.useRealTimers();
    native.kind = 'hero';
    native.read.mockReset();
    native.close.mockReset();
    native.focus.mockReset();
  });
  for (const scope of ['owned', 'room:guest'])
    it(`renders the accepted ${scope} hero without mutation controls`, async () => {
      native.read.mockResolvedValue(frame(scope));
      render(<AuxiliaryApp />);
      expect(await screen.findByText('Mira')).toBeInTheDocument();
      expect(screen.getByText('Dungeon Master AI')).toBeInTheDocument();
      expect(screen.getByText('Read-only view')).toBeInTheDocument();
      expect(screen.getByText('9 / 12')).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /Rest|onboarding/i })).not.toBeInTheDocument();
    });
  it('renders only published journal and sanitizes its HTML', async () => {
    native.kind = 'journal';
    native.read.mockResolvedValue({
      ...frame('room:guest', false),
      journal: [
        {
          id: 'entry',
          campaign_id: campaign,
          chapter: 'Discovered',
          entry_html: '<p>Shared clue</p><script>danger()</script>',
          created_at: '2026-10-14T00:00:00Z',
        },
      ],
    });
    const { container } = render(<AuxiliaryApp />);
    expect(await screen.findByText('Shared clue')).toBeInTheDocument();
    expect(screen.queryByText('My own biography')).not.toBeInTheDocument();
    expect(container.querySelector('script')).toBeNull();
  });
  it('renders the owned preview and clears it after scope revocation', async () => {
    native.read.mockResolvedValue(portraitFrame());
    const { container } = render(<AuxiliaryApp />);
    expect(await screen.findByText('Mira')).toBeInTheDocument();
    expect(container.querySelector('.dm-char-portrait-art')).toHaveAttribute('src', preview);
    native.read.mockResolvedValue(null);
    await waitFor(() => expect(container.querySelector('.dm-char-portrait-art')).toBeNull(), {
      timeout: 2500,
    });
  });
  it('rejects a preview bound to a different asset', async () => {
    const value = portraitFrame();
    value.portrait.assetId = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd';
    native.read.mockResolvedValue(value);
    render(<AuxiliaryApp />);
    await act(async () => {});
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.queryByText('Mira')).not.toBeInTheDocument();
  });
  it('explains unavailable owned art while displaying the class fallback', async () => {
    native.read.mockResolvedValue({ ...portraitFrame(), portrait: null });
    const { container } = render(<AuxiliaryApp />);
    expect(await screen.findByText('Mira')).toBeInTheDocument();
    expect(screen.getByText('Portrait unavailable. Showing class artwork.')).toBeInTheDocument();
    expect(container.querySelector('.dm-char-portrait-art')?.getAttribute('src')).not.toBe(preview);
  });
  it('clears the displayed hero when the lease is revoked', async () => {
    native.read.mockResolvedValue(frame());
    render(<AuxiliaryApp />);
    expect(await screen.findByText('Mira')).toBeInTheDocument();
    native.read.mockResolvedValue({
      ...frame(),
      sequence: 2,
      ready: false,
      hero: null,
      journal: [],
    });
    await waitFor(() => expect(screen.queryByText('Mira')).not.toBeInTheDocument(), {
      timeout: 2500,
    });
    expect(screen.getByRole('status')).toBeInTheDocument();
  });
  it.each([
    null,
    { ...frame(), actorId: 'other-actor' },
    {
      ...frame(),
      hero: null,
      ready: false,
      journal: [
        {
          id: 'wrong',
          campaign_id: 'other',
          entry_html: 'private',
          chapter: null,
          created_at: '2026-10-14',
        },
      ],
    },
  ])('shows unavailable for missing or invalid scope', async (value) => {
    native.read.mockResolvedValue(value);
    render(<AuxiliaryApp />);
    await act(async () => {});
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.queryByText('Mira')).not.toBeInTheDocument();
  });
});
