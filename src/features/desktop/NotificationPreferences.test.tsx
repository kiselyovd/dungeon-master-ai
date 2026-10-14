import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import { notificationPreferences, setNotifications } from '../../api/notifications';
import i18n from '../../i18n';
import { NotificationPreferences } from './NotificationPreferences';

vi.mock('../../api/notifications', () => ({
  notificationPreferences: vi.fn(),
  setNotifications: vi.fn(),
}));
const prefs = {
  enabled: false,
  appScopeId: '00000000-0000-4000-8000-000000000001',
  permission: 'granted' as const,
};
beforeEach(async () => {
  vi.clearAllMocks();
  vi.mocked(notificationPreferences).mockResolvedValue(prefs);
  vi.mocked(setNotifications).mockResolvedValue({ ...prefs, enabled: true });
  await i18n.changeLanguage('en');
});
it('requires explicit opt-in and saves the UI language with it', async () => {
  const user = userEvent.setup();
  render(<NotificationPreferences />);
  const box = await screen.findByRole('checkbox', { name: 'Notify me when the app is inactive' });
  await waitFor(() => expect(box).toBeEnabled());
  expect(box).not.toBeChecked();
  expect(setNotifications).not.toHaveBeenCalled();
  await user.click(box);
  await waitFor(() => expect(box).toBeChecked());
  expect(setNotifications).toHaveBeenCalledWith(true, 'en');
});
it('reports OS refusal without claiming successful delivery', async () => {
  vi.mocked(notificationPreferences).mockResolvedValue({ ...prefs, permission: 'denied' });
  render(<NotificationPreferences />);
  expect(await screen.findByRole('status')).toHaveTextContent('Windows has disabled');
  expect(setNotifications).not.toHaveBeenCalled();
});
it('keeps the preference unchanged when native persistence fails', async () => {
  const user = userEvent.setup();
  vi.mocked(setNotifications).mockRejectedValue(Error('native_notification_storage_unavailable'));
  render(<NotificationPreferences />);
  const box = await screen.findByRole('checkbox');
  await waitFor(() => expect(box).toBeEnabled());
  await user.click(box);
  expect(await screen.findByRole('alert')).toHaveTextContent('Could not update');
  expect(box).not.toBeChecked();
});
it('shows unsupported platform state without a working toggle', async () => {
  vi.mocked(notificationPreferences).mockResolvedValue({ ...prefs, permission: 'unsupported' });
  render(<NotificationPreferences />);
  expect(await screen.findByRole('status')).toHaveTextContent('unavailable on this platform');
  expect(screen.getByRole('checkbox')).toBeDisabled();
});
