import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import { CampaignCreate } from '../CampaignCreate';

it('creates the executable bundled adventure instead of a future-source dead end', async () => {
  await i18n.changeLanguage('en');
  const user = userEvent.setup();
  const create = vi.fn(async () => undefined);
  render(<CampaignCreate busy={false} onBack={vi.fn()} onCreate={create} />);
  await user.click(screen.getByRole('radio', { name: /Prepared adventure/ }));
  await user.click(screen.getByRole('button', { name: 'Continue' }));
  expect(screen.getByRole('heading', { name: 'The Bell Beneath the Water' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Continue' }));
  await user.click(screen.getByRole('button', { name: 'Continue' }));
  await user.click(screen.getByRole('button', { name: 'Create campaign' }));
  expect(create).toHaveBeenCalledWith(
    expect.objectContaining({ seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' } }),
  );
});
