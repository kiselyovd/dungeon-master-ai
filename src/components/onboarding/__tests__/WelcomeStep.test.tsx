import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { WelcomeStep } from '../steps/WelcomeStep';

describe('WelcomeStep', () => {
  it('renders the living tabletop welcome artwork', () => {
    render(<WelcomeStep titleId="welcome-title" onNext={vi.fn()} />);

    expect(screen.getByTestId('onboarding-hero-art')).toHaveAttribute(
      'src',
      expect.stringContaining('onboarding'),
    );
  });
});
