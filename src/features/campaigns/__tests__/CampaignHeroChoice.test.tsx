import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import { useStore } from '../../../state/useStore';
import { CampaignHeroChoice } from '../CampaignHeroChoice';

const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
beforeEach(async () => {
  useStore.setState(useStore.getInitialState());
  await i18n.changeLanguage('en');
});
it('keeps prepared hero selection private to the draft until the server confirms it', async () => {
  const commit = vi.fn(async () => {
    throw new Error('offline');
  });
  render(
    <CampaignHeroChoice
      campaignId={id}
      pending={false}
      onCommit={commit}
      onCustom={vi.fn()}
      onBack={vi.fn()}
    />,
  );
  await userEvent.click(await screen.findByRole('radio', { name: /Wizard/ }));
  await userEvent.type(screen.getByLabelText('Hero name'), 'Mira');
  await userEvent.click(screen.getByRole('button', { name: 'Enter first scene' }));
  expect(await screen.findByRole('alert')).toBeVisible();
  expect(useStore.getState().pc.heroClass).toBeNull();
  expect(useStore.getState().charCreation.name).toBe('Mira');
  expect(commit).toHaveBeenCalledWith(
    expect.objectContaining({
      heroClass: 'wizard',
      name: 'Mira',
      spells: expect.objectContaining({ cantrips: expect.arrayContaining(['fire-bolt']) }),
    }),
  );
});
it('resets a draft from another campaign without displaying that hero', async () => {
  useStore
    .getState()
    .charCreation.bindScope({ kind: 'campaign', campaignId: 'B', characterId: null });
  useStore.getState().charCreation.setDraftField('name', 'Other hero');
  render(
    <CampaignHeroChoice
      campaignId={id}
      pending={false}
      onCommit={vi.fn()}
      onCustom={vi.fn()}
      onBack={vi.fn()}
    />,
  );
  expect(await screen.findByLabelText('Hero name')).toHaveValue('');
  expect(screen.queryByDisplayValue('Other hero')).toBeNull();
});
