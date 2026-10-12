import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CampaignSummary } from '../../../api/contracts/campaign';
import i18n from '../../../i18n';
import { CampaignHome } from '../CampaignHome';

const campaigns: CampaignSummary[] = [
  {
    id: '62ba8579-904b-4460-a68a-df6268ed404a',
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    title: 'Серебряная долина',
    premise: 'Древняя дорога проснулась',
    last_session_id: 'b5271021-b82c-40c4-aa9f-554af0d79b22',
    revision: 4,
    updated_at: '2026-08-13T03:21:00.000Z',
  },
  {
    id: '506ce5d3-1df2-4235-85d4-a4d56cf9fc31',
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    title: 'Пепельный тракт',
    premise: 'На дороге исчезают караваны',
    last_session_id: '15dde980-87c1-4e36-ac18-7ed29419c337',
    revision: 2,
    updated_at: '2026-08-12T03:21:00.000Z',
  },
];

beforeEach(async () => {
  await i18n.changeLanguage('ru');
});

describe('CampaignHome', () => {
  it('shows current place and actual timestamps rather than a revision posing as a chapter', () => {
    const known = campaigns[0];
    if (!known) throw new Error('campaign fixture missing');
    render(
      <CampaignHome
        campaigns={[{ ...known, current_location: { kind: 'authored', location: 'archive' } }]}
        loading={false}
        onContinue={vi.fn()}
        onNew={vi.fn()}
      />,
    );
    expect(screen.getByText('Затопленный архив')).toBeVisible();
    expect(screen.getByText(/^Сохранено:/)).toBeVisible();
    expect(screen.queryByText(/Глава|srd-5\.2\.1/i)).not.toBeInTheDocument();
  });
  it('turns an empty library into a clear invitation', () => {
    render(<CampaignHome campaigns={[]} loading={false} onContinue={vi.fn()} onNew={vi.fn()} />);
    expect(screen.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Кампаний пока нет' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Новая кампания' })).toBeVisible();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('features the latest campaign and keeps keyboard actions in reading order', async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    const onNew = vi.fn();
    render(
      <CampaignHome campaigns={campaigns} loading={false} onContinue={onContinue} onNew={onNew} />,
    );

    expect(screen.getByRole('heading', { name: 'Серебряная долина' })).toBeVisible();
    expect(screen.getByText('Пепельный тракт')).toBeVisible();
    await user.tab();
    expect(screen.getByRole('button', { name: 'Продолжить кампанию' })).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onContinue).toHaveBeenCalledWith(campaigns[0]?.id);
    await user.tab();
    expect(screen.getByRole('button', { name: 'Новая кампания' })).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onNew).toHaveBeenCalledOnce();
  });
});
