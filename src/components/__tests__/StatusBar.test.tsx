import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { StatusBar } from '../StatusBar';

describe('StatusBar modality indicators', () => {
  it('renders the image preset when image is enabled', () => {
    render(
      <StatusBar
        provider="Anthropic"
        model="haiku"
        image={{ enabled: true, label: 'balanced' }}
        video={{ enabled: false, label: 'off' }}
      />,
    );
    expect(screen.getByText(/balanced/i)).toBeInTheDocument();
  });

  it('renders "off" when image is disabled', () => {
    render(
      <StatusBar
        provider="Anthropic"
        model="haiku"
        image={{ enabled: false, label: 'off' }}
        video={{ enabled: false, label: 'off' }}
      />,
    );
    const offs = screen.getAllByText(/^off$/i);
    expect(offs.length).toBeGreaterThanOrEqual(1);
  });

  it('renders the video mode when video is enabled', () => {
    render(
      <StatusBar
        provider="Anthropic"
        model="haiku"
        image={{ enabled: false, label: 'off' }}
        video={{ enabled: true, label: 'live' }}
      />,
    );
    expect(screen.getByText(/live/i)).toBeInTheDocument();
  });

  it('calls onOpenSettings with "image" tab when image pill clicked', async () => {
    const onOpen = vi.fn();
    render(
      <StatusBar
        provider="Anthropic"
        model="haiku"
        image={{ enabled: true, label: 'balanced' }}
        video={{ enabled: false, label: 'off' }}
        onOpenSettings={onOpen}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: /image gen settings/i }));
    expect(onOpen).toHaveBeenCalledWith('image');
  });

  it('calls onOpenSettings with "video" tab when video pill clicked', async () => {
    const onOpen = vi.fn();
    render(
      <StatusBar
        provider="Anthropic"
        model="haiku"
        image={{ enabled: false, label: 'off' }}
        video={{ enabled: true, label: 'live' }}
        onOpenSettings={onOpen}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: /video gen settings/i }));
    expect(onOpen).toHaveBeenCalledWith('video');
  });

  it('calls onOpenSettings with "chat" when chat indicator clicked', async () => {
    const onOpen = vi.fn();
    render(
      <StatusBar
        provider="Anthropic"
        model="haiku"
        image={{ enabled: false, label: 'off' }}
        video={{ enabled: false, label: 'off' }}
        onOpenSettings={onOpen}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: /chat provider settings/i }));
    expect(onOpen).toHaveBeenCalledWith('chat');
  });
});
