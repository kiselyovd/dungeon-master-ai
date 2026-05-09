import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import '../../i18n';
import { ScenePill } from '../ScenePill';

describe('ScenePill', () => {
  it('renders the scene name and step text when a scene is set', () => {
    render(<ScenePill scene={{ name: 'Crimson Sanctuary', stepCounter: 3 }} />);
    expect(screen.getByText('Crimson Sanctuary')).toBeInTheDocument();
    expect(screen.getByText(/Step 3/)).toBeInTheDocument();
    // Pill is announced as a polite status region for assistive tech.
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('renders nothing when scene is null so the titlebar centre stays empty', () => {
    const { container } = render(<ScenePill scene={null} />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByRole('status')).toBeNull();
  });
});
