import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ImageLightboxModal } from '../ImageLightboxModal';
import '../../i18n';

describe('ImageLightboxModal', () => {
  it('renders the image with the given src + alt', () => {
    render(<ImageLightboxModal src="data:image/png;base64,aGk=" alt="x" onClose={() => {}} />);
    const img = screen.getByTestId('lightbox-image') as HTMLImageElement;
    expect(img.getAttribute('src')).toBe('data:image/png;base64,aGk=');
    expect(img.alt).toBe('x');
  });

  it('closes when Escape is pressed', () => {
    const onClose = vi.fn();
    render(<ImageLightboxModal src="x" alt="" onClose={onClose} />);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes when the backdrop is clicked', () => {
    const onClose = vi.fn();
    render(<ImageLightboxModal src="x" alt="" onClose={onClose} />);
    fireEvent.click(screen.getByTestId('lightbox-backdrop'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does NOT close when the image frame is clicked', () => {
    const onClose = vi.fn();
    render(<ImageLightboxModal src="x" alt="" onClose={onClose} />);
    fireEvent.click(screen.getByTestId('lightbox-image'));
    expect(onClose).not.toHaveBeenCalled();
  });
});
