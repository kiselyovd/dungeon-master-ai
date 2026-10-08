import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const catalog = JSON.parse(
  readFileSync(
    new URL('../crates/app-domain/src/compendium/data/srd_5_2_1.yaml', import.meta.url),
    'utf8',
  ),
);

test('production wizard previews equipped armor and persists an explicit carried-only choice', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: null,
    active_campaign_id: '2c7d0d90-e970-47a7-aac4-66a147f96e58',
    active_session_id: '53a7fa02-cfb6-4717-b7fe-e6408f81a569',
    active_provider: 'local-mistralrs',
    ui_language: 'en',
    char_creation_draft: {
      rulesetId: 'srd-5.2.1',
      classId: 'fighter',
      raceId: 'human',
      abilities: { str: 12, dex: 14, con: 16, int: 10, wis: 10, cha: 10 },
      equipmentMode: 'gold',
      equipmentInventory: [
        { id: 'chain-mail', name: 'Chain Mail', count: 1 },
        { id: 'shield', name: 'Shield', count: 1 },
      ],
      goldRemaining: 0,
      activeTab: 'equipment',
    },
  });
  await page.route('**/srd/**', (route) => {
    const key = new URL(route.request().url()).pathname.split('/').pop()?.replace('-', '_') ?? '';
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(catalog[key] ?? []),
    });
  });
  const open = async () => {
    await expect(page.locator('.dm-wizard')).toBeVisible();
    await page.getByRole('tab', { name: /Equipment/i }).click();
  };
  await page.goto('/');
  await open();
  const wizard = page.locator('.dm-wizard');
  await expect(wizard.getByText('Armor Class: 18', { exact: true })).toBeVisible();
  await expect(
    wizard.getByText('Insufficient Strength: speed reduced by 10 feet.', { exact: true }),
  ).toBeVisible();
  await wizard.getByRole('combobox', { name: 'Worn armor' }).click();
  await page.getByRole('option', { name: 'No armor worn', exact: true }).click();
  await wizard.getByRole('checkbox', { name: 'Use a shield' }).uncheck();
  await expect(wizard.getByText('Armor Class: 12', { exact: true })).toBeVisible();
  await page.reload();
  await open();
  await expect(wizard.getByText('Armor Class: 12', { exact: true })).toBeVisible();
  await expect(wizard.getByRole('checkbox', { name: 'Use a shield' })).not.toBeChecked();
});

test('production wizard explains untrained armor and shield and retains the explicit choice', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: null,
    active_campaign_id: '2c7d0d90-e970-47a7-aac4-66a147f96e58',
    active_session_id: '53a7fa02-cfb6-4717-b7fe-e6408f81a569',
    active_provider: 'local-mistralrs',
    ui_language: 'en',
    char_creation_draft: {
      rulesetId: 'srd-5.2.1',
      classId: 'wizard',
      raceId: 'human',
      abilities: { str: 12, dex: 14, con: 16, int: 10, wis: 10, cha: 10 },
      equipmentMode: 'gold',
      equipmentInventory: [
        { id: 'chain-mail', name: 'Chain Mail', count: 1 },
        { id: 'shield', name: 'Shield', count: 1 },
      ],
      goldRemaining: 0,
      activeTab: 'equipment',
    },
  });
  await page.route('**/srd/**', (route) => {
    const key = new URL(route.request().url()).pathname.split('/').pop()?.replace('-', '_') ?? '';
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(catalog[key] ?? []),
    });
  });
  const open = async () => {
    await expect(page.locator('.dm-wizard')).toBeVisible();
    await page.getByRole('tab', { name: /Equipment/i }).click();
  };
  await page.goto('/');
  await open();
  const wizard = page.locator('.dm-wizard');
  await expect(wizard.getByText('Armor Class: 12', { exact: true })).toBeVisible();
  await expect(wizard.getByRole('checkbox', { name: 'Use a shield' })).not.toBeChecked();
  await wizard.getByRole('combobox', { name: 'Worn armor' }).click();
  await page.getByRole('option', { name: 'Chain Mail', exact: true }).click();
  await wizard.getByRole('checkbox', { name: 'Use a shield' }).check();
  await expect(wizard.getByText('Armor Class: 16', { exact: true })).toBeVisible();
  await expect(
    wizard.getByText(
      'No training with this armor: Strength and Dexterity tests have disadvantage, and you cannot cast spells.',
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    wizard.getByText('No shield training: this shield gives no Armor Class bonus.', {
      exact: true,
    }),
  ).toBeVisible();
  await page.reload();
  await open();
  await expect(wizard.getByText('Armor Class: 16', { exact: true })).toBeVisible();
  await expect(wizard.getByRole('checkbox', { name: 'Use a shield' })).toBeChecked();
  await expect(
    wizard.getByText(
      'No training with this armor: Strength and Dexterity tests have disadvantage, and you cannot cast spells.',
      { exact: true },
    ),
  ).toBeVisible();
});
