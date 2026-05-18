import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { LicenseRestrictedBanner } from '../LicenseRestrictedBanner';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => (opts ? `${key}:${opts.name}` : key),
  }),
}));

describe('LicenseRestrictedBanner', () => {
  it('renders nothing when modality is null', () => {
    const { container } = render(
      <LicenseRestrictedBanner modality={null} activePresetName={null} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders nothing when activePresetName is null', () => {
    const { container } = render(
      <LicenseRestrictedBanner modality="image" activePresetName={null} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders alert with preset name interpolation', () => {
    render(<LicenseRestrictedBanner modality="image" activePresetName="Nunchaku FLUX" />);
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent(/Nunchaku FLUX/);
  });
});
