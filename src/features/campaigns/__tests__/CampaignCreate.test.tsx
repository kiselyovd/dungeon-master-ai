import { LazyStore } from '@tauri-apps/plugin-store';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import { CampaignCreate } from '../CampaignCreate';

beforeEach(async () => {
  await i18n.changeLanguage('en');
  await new LazyStore('campaign-preparation.json').delete('draft');
});
it('creates a prepared adventure with one real action and no empty handoffs', async () => {
  const create = vi.fn(async () => undefined);
  render(<CampaignCreate busy={false} onBack={vi.fn()} onCreate={create} />);
  await userEvent.click(await screen.findByRole('button', { name: 'Choose hero' }));
  expect(create).toHaveBeenCalledWith(
    expect.objectContaining({ seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' } }),
  );
  expect(screen.queryByText(/PDF|DOCX/)).toBeNull();
});
it('retains a custom opening draft after leaving and reopening the form', async () => {
  const user = userEvent.setup();
  const create = vi.fn(async () => undefined);
  const view = render(<CampaignCreate busy={false} onBack={vi.fn()} onCreate={create} />);
  await screen.findByRole('button', { name: 'Choose hero' });
  await user.click(screen.getByRole('radio', { name: /Create together/ }));
  await user.type(screen.getByLabelText('Campaign title'), 'The road');
  await user.type(screen.getByLabelText('Opening scene'), 'A bridge in the rain.');
  await user.click(screen.getByRole('button', { name: 'Back' }));
  view.unmount();
  render(<CampaignCreate busy={false} onBack={vi.fn()} onCreate={create} />);
  expect(await screen.findByDisplayValue('The road')).toBeVisible();
  expect(screen.getByDisplayValue('A bridge in the rain.')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Choose hero' }));
  expect(create).toHaveBeenCalledWith(
    expect.objectContaining({ title: 'The road', premise: 'A bridge in the rain.' }),
  );
});
it('keeps the draft and an actionable error after rejected creation', async () => {
  render(
    <CampaignCreate
      busy={false}
      onBack={vi.fn()}
      onCreate={vi.fn(async () => {
        throw new Error('offline');
      })}
    />,
  );
  await userEvent.click(await screen.findByRole('button', { name: 'Choose hero' }));
  expect(await screen.findByRole('alert')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Choose hero' })).toBeEnabled();
});
