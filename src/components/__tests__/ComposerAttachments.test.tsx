import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { StagedImage } from '../../state/chat';
import { ComposerAttachments } from '../ComposerAttachments';
import '../../i18n';

const sample: StagedImage[] = [
  {
    mime: 'image/png',
    dataUrl: 'data:image/png;base64,aGk=',
    name: 'a.png',
    sizeBytes: 100,
  },
];

describe('ComposerAttachments', () => {
  it('renders a thumbnail for each staged image', () => {
    render(<ComposerAttachments items={sample} onRemove={() => {}} />);
    expect(screen.getByRole('img')).toBeTruthy();
    expect(screen.getByRole('list')).toBeTruthy();
  });

  it('calls onRemove with the index when × is clicked', () => {
    const onRemove = vi.fn();
    render(<ComposerAttachments items={sample} onRemove={onRemove} />);
    fireEvent.click(screen.getByLabelText(/remove image|удалить изображение/i));
    expect(onRemove).toHaveBeenCalledWith(0);
  });

  it('renders nothing when items is empty', () => {
    const { container } = render(<ComposerAttachments items={[]} onRemove={() => {}} />);
    expect(container.firstChild).toBeNull();
  });
});
