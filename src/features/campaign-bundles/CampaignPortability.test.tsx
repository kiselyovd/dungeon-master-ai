import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { CampaignPortability } from './CampaignPortability';

const api = vi.hoisted(() => ({
  pick: vi.fn(),
  confirm: vi.fn(),
  cancel: vi.fn(async () => {}),
  export: vi.fn(),
  reveal: vi.fn(async () => {}),
}));
vi.mock('../../api/campaignBundles', () => ({
  nativeBundlesAvailable: () => true,
  pickCampaignImport: api.pick,
  confirmCampaignImport: api.confirm,
  cancelCampaignImport: api.cancel,
  exportCampaignFile: api.export,
  revealCampaignFile: api.reveal,
  bundleFailureCode: (error: unknown) =>
    typeof error === 'string' ? error : 'native_bundle_request_failed',
}));
const original = '00000000-0000-4000-8000-000000000001';
const imported = '00000000-0000-4000-8000-000000000002';
const choice = {
  operationId: '00000000-0000-4000-8000-000000000003',
  fileName: 'Кампания.dmai',
  preview: {
    kind: 'full_campaign_backup',
    title: 'Imported tale',
    character_count: 2,
    message_count: 10,
    media_count: 1,
    private_backup: true,
  },
};
const result = {
  campaign_id: imported,
  session_id: '00000000-0000-4000-8000-000000000004',
  revision: 1,
  remapped_ids: {},
};
const campaigns = [{ id: original, title: 'Original campaign' }];
beforeEach(async () => {
  vi.clearAllMocks();
  api.pick.mockReset().mockResolvedValue(choice);
  api.confirm.mockReset().mockResolvedValue(result);
  api.export.mockReset().mockResolvedValue({ fileName: 'Saved.dmai', bytes: 100 });
  await i18n.changeLanguage('en');
});
it('previews before creating a new campaign and requires explicit opening', async () => {
  const user = userEvent.setup();
  const refresh = vi.fn();
  const open = vi.fn();
  render(
    <CampaignPortability
      scopeId="library"
      campaigns={campaigns}
      onImported={refresh}
      onOpen={open}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Import campaign' }));
  expect(await screen.findByText('Imported tale')).toBeVisible();
  expect(api.confirm).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Create a new campaign' }));
  expect(
    await screen.findByText('The new campaign was created. Open it when ready.'),
  ).toBeVisible();
  expect(api.confirm).toHaveBeenCalledWith(choice.operationId);
  expect(refresh).toHaveBeenCalledWith(result);
  expect(open).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Open imported campaign' }));
  expect(open).toHaveBeenCalledWith(imported);
});
it('cancel discards the prepared file without committing', async () => {
  const user = userEvent.setup();
  render(<CampaignPortability scopeId="library" campaigns={campaigns} />);
  await user.click(screen.getByRole('button', { name: 'Import campaign' }));
  await screen.findByText('Imported tale');
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(api.cancel).toHaveBeenCalledWith(choice.operationId);
  expect(api.confirm).not.toHaveBeenCalled();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('native picker cancellation makes no campaign changes', async () => {
  api.pick.mockResolvedValue(null);
  const user = userEvent.setup();
  render(<CampaignPortability scopeId="library" campaigns={campaigns} />);
  await user.click(screen.getByRole('button', { name: 'Import campaign' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Import campaign' })).toBeEnabled(),
  );
  expect(api.confirm).not.toHaveBeenCalled();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('a guest can export only its dossier and reveal the resulting file', async () => {
  const user = userEvent.setup();
  render(<CampaignPortability scopeId="room" campaigns={campaigns} guest canImport={false} />);
  expect(screen.queryByRole('button', { name: 'Import campaign' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Export campaign' }));
  expect(screen.getByLabelText('File contents')).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Choose file and save' }));
  expect(await screen.findByText('Saved: Saved.dmai')).toBeVisible();
  expect(api.export).toHaveBeenCalledWith(original, 'player_dossier', 'en');
  await user.click(screen.getByRole('button', { name: 'Show file in folder' }));
  expect(api.reveal).toHaveBeenCalledOnce();
});
it('unknown delivery keeps the same operation available for a safe retry', async () => {
  api.confirm.mockRejectedValueOnce('native_bundle_delivery_unknown').mockResolvedValue(result);
  const user = userEvent.setup();
  render(<CampaignPortability scopeId="library" campaigns={campaigns} />);
  await user.click(screen.getByRole('button', { name: 'Import campaign' }));
  await screen.findByText('Imported tale');
  await user.click(screen.getByRole('button', { name: 'Create a new campaign' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('The result was not confirmed');
  await user.click(screen.getByRole('button', { name: 'Create a new campaign' }));
  await screen.findByText('The new campaign was created. Open it when ready.');
  expect(api.confirm.mock.calls).toEqual([[choice.operationId], [choice.operationId]]);
});
it('a late picker result is discarded after the current scope changes', async () => {
  let resolve: ((value: typeof choice) => void) | undefined;
  api.pick.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const user = userEvent.setup();
  const view = render(<CampaignPortability scopeId="room-a" campaigns={campaigns} />);
  await user.click(screen.getByRole('button', { name: 'Import campaign' }));
  view.rerender(<CampaignPortability scopeId="room-b" campaigns={campaigns} />);
  await act(async () => {
    resolve?.(choice);
  });
  expect(api.cancel).toHaveBeenCalledWith(choice.operationId);
  expect(api.confirm).not.toHaveBeenCalled();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('refreshing the library after success does not dismiss the import result', async () => {
  const user = userEvent.setup();
  const view = render(<CampaignPortability scopeId="library" campaigns={campaigns} />);
  await user.click(screen.getByRole('button', { name: 'Import campaign' }));
  await screen.findByText('Imported tale');
  await user.click(screen.getByRole('button', { name: 'Create a new campaign' }));
  await screen.findByText('The new campaign was created. Open it when ready.');
  view.rerender(
    <CampaignPortability
      scopeId="library"
      campaigns={[{ id: imported, title: 'Imported tale' }, ...campaigns]}
    />,
  );
  expect(screen.getByText('The new campaign was created. Open it when ready.')).toBeVisible();
});
it('selects the campaign when the library finishes loading after mount', async () => {
  const user = userEvent.setup();
  const view = render(<CampaignPortability scopeId="library" campaigns={[]} />);
  view.rerender(<CampaignPortability scopeId="library" campaigns={campaigns} />);
  await user.click(screen.getByRole('button', { name: 'Export campaign' }));
  await waitFor(() => expect(screen.getByLabelText('Campaign')).toHaveValue(original));
  await user.click(screen.getByRole('button', { name: 'Choose file and save' }));
  expect(await screen.findByText('Saved: Saved.dmai')).toBeVisible();
  expect(api.export).toHaveBeenCalledWith(original, 'full_campaign_backup', 'en');
});
