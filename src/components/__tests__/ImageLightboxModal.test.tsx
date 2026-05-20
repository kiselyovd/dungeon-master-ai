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

  // B6: close button inside the frame at top-right with autoFocus
  it('renders close button inside the image frame and it receives focus on open', () => {
    render(<ImageLightboxModal src="data:image/png;base64,aGk=" alt="test" onClose={() => {}} />);
    const closeBtn = screen.getByRole('button', { name: /close/i });
    expect(closeBtn).toBeInTheDocument();
    // The close button must be inside the frame, not outside it
    const image = screen.getByTestId('lightbox-image');
    // Both share a parent .frame div; close button must be a sibling of the image
    expect(image.parentElement).toContainElement(closeBtn);
    // autoFocus - the close button should be the active element after render
    expect(document.activeElement).toBe(closeBtn);
  });
});
