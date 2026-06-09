import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import '../i18n';
import { ToolCallCard } from './ToolCallCard';

const BASE = {
  id: 'call_img_1',
  args: { prompt: 'a dark dungeon', width: 512, height: 512 },
  isError: false,
  round: 2,
  timestamp: '2026-06-09T10:00:00Z',
  handledBy: 'image-provider',
};

const pendingMapEntry = {
  ...BASE,
  toolName: 'generate_map',
  result: null,
};

const pendingIllustrationEntry = {
  ...BASE,
  id: 'call_img_2',
  toolName: 'generate_illustration',
  result: null,
};

const TEST_DATA_URL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';

const settledMapEntry = {
  ...BASE,
  toolName: 'generate_map',
  result: { ok: true },
  imageDataUrl: TEST_DATA_URL,
  imageKind: 'map' as const,
};

const settledIllustrationEntry = {
  ...BASE,
  id: 'call_img_3',
  toolName: 'generate_illustration',
  result: { ok: true },
  imageDataUrl: TEST_DATA_URL,
  imageKind: 'chat' as const,
};

describe('ToolCallCard - image tools', () => {
  it('pending generate_map shows drawing indicator and NOT raw args JSON', () => {
    render(<ToolCallCard entry={pendingMapEntry} />);
    expect(screen.getByTestId('tool-drawing')).toBeInTheDocument();
    // Raw args JSON must NOT appear
    expect(screen.queryByText(/"prompt"/)).toBeNull();
  });

  it('pending generate_illustration shows drawing indicator and NOT raw args JSON', () => {
    render(<ToolCallCard entry={pendingIllustrationEntry} />);
    expect(screen.getByTestId('tool-drawing')).toBeInTheDocument();
    expect(screen.queryByText(/"prompt"/)).toBeNull();
  });

  it('settled generate_map with imageDataUrl renders an img with the data URL', () => {
    render(<ToolCallCard entry={settledMapEntry} />);
    const img = screen.getByRole('img', { name: /generated image|image_alt/i });
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', TEST_DATA_URL);
    // No drawing indicator once settled
    expect(screen.queryByTestId('tool-drawing')).toBeNull();
    // No raw args JSON
    expect(screen.queryByText(/"prompt"/)).toBeNull();
  });

  it('settled generate_illustration with imageDataUrl renders an img with the data URL', () => {
    render(<ToolCallCard entry={settledIllustrationEntry} />);
    const img = screen.getByRole('img', { name: /generated image|image_alt/i });
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', TEST_DATA_URL);
  });
});
