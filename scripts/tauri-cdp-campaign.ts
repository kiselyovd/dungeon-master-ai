/**
 * Complete real-Tauri campaign acceptance over raw WebView2 CDP.
 *
 * The driver uses only rendered UI, Tauri commands and the loopback HTTP API.
 * It never imports the Zustand store and never writes application state from
 * the page. Logs and artifacts contain checkpoint metadata only: no prompts,
 * narration, keys, base64 media or full persistence rows.
 */
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

import {
  CdpClient,
  captureScreenshot,
  defaultArtifactPath,
  evaluateInPage,
  findAppPageWebSocket,
  waitForPageCondition,
} from './lib/cdp';
import { evaluateDmReply, type DmReplySnapshot } from './lib/dm-reply';
import {
  evaluateCampaignEvidence,
  isChatScrollContained,
  isCombatSceneVisible,
  resolveJournalEvidenceId,
  resolveCampaignBootstrapAction,
  resolveDmResponseTimeoutMs,
  resolveRuntimeTimeoutMs,
  type CampaignEvidence,
  type CombatSceneVisibilitySnapshot,
  type SafeCampaignEvidenceFile,
} from './lib/tauri-campaign';

const artifactRoot = dirname(defaultArtifactPath('full-campaign/campaign-evidence.json'));
const evidencePath = `${artifactRoot}/campaign-evidence.json`;
const runId = crypto.randomUUID().slice(0, 8);
// Keep the visible identity short and deterministic: small local tool-calling
// models have been observed to drop a random suffix, which makes the
// server-authoritative combat token no longer match the player character.
// The isolated Tauri profile and runId-backed evidence IDs still prevent runs
// from sharing state.
const heroName = 'CDP Hero';
const campaignPrompt = [
  'Run a short acceptance-test scene and call each requested tool exactly once before narration.',
  `Call set_scene for Moonlit Gate, generate_illustration, generate_map, remember_npc, journal_append, then start_combat.`,
  `For start_combat use two entries: ${heroName} with roll 20, hp 12, max_hp 12, ac 14; and Gate Goblin with roll 1, hp 7, max_hp 7, ac 13.`,
  'Use concrete classic-fantasy visual prompts. Finish with one short narration sentence.',
].join(' ');
const endCombatPrompt =
  'Tool execution is required: call end_combat with an empty object now. Do not narrate before the tool succeeds. After the tool result, answer with one short sentence.';
const endCombatRetryPrompt =
  'The encounter is still active. You must call end_combat with {} in this turn; prose without that tool call is invalid. After it succeeds, answer with one short sentence.';
const startCombatRetryPrompt =
  `Tool execution is required: call start_combat now with exactly two initiative_entries: ${heroName} roll 20 and Gate Goblin roll 1. Do not narrate before the tool succeeds.`;
const journalPrompt =
  'Call journal_append exactly once with chapter Acceptance and a short entry_html about the Moonlit Gate. Then answer with one short sentence.';

const sleep = (milliseconds: number) =>
  new Promise<void>((resolvePromise) => setTimeout(resolvePromise, milliseconds));

function logCheckpoint(item: CampaignEvidence): void {
  const safe = { ...item } as Record<string, unknown>;
  console.log(`PASS checkpoint=${item.checkpoint} ${JSON.stringify(safe)}`);
}

async function backendPort(cdp: CdpClient): Promise<number> {
  const deadline = Date.now() + 120_000;
  let port: number | null = null;
  while (Date.now() < deadline) {
    port = await evaluateInPage<number | null>(
      cdp,
      `(async () => {
        const invoke = window.__TAURI_INTERNALS__?.invoke;
        if (typeof invoke !== 'function') return null;
        try { return await invoke('backend_port'); } catch { return null; }
      })()`,
    );
    if (Number.isInteger(port) && port !== null && port > 0) return port;
    await sleep(500);
  }
  throw new Error(`backend_port_invalid:${String(port)}`);
}

async function runtimeStatus(port: number): Promise<{
  llm?: { state?: string; port?: number };
  image?: { state?: string; port?: number };
  model_id?: string;
}> {
  const response = await fetch(`http://127.0.0.1:${port}/local/runtime/status`);
  if (!response.ok) throw new Error(`runtime_status_http_${response.status}`);
  return response.json();
}

async function waitForLlmReady(port: number, timeoutMs: number): Promise<ReturnType<typeof runtimeStatus>> {
  const deadline = Date.now() + timeoutMs;
  let status: Awaited<ReturnType<typeof runtimeStatus>> = {};
  while (Date.now() < deadline) {
    try {
      status = await runtimeStatus(port);
      if (status.llm?.state === 'ready') return status;
    } catch {
      // Runtime is allowed to be temporarily unavailable while Tauri replaces it.
    }
    await sleep(2_000);
  }
  throw new Error(`local_llm_not_ready:${status.llm?.state ?? 'unknown'}`);
}

async function clickFirst(cdp: CdpClient, selector: string): Promise<boolean> {
  return evaluateInPage<boolean>(
    cdp,
    `(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (!(element instanceof HTMLElement)) return false;
      element.click();
      return true;
    })()`,
  );
}

async function openSettings(cdp: CdpClient): Promise<void> {
  if (await evaluateInPage<boolean>(cdp, `Boolean(document.querySelector('#settings-form'))`)) {
    return;
  }
  const opened = await evaluateInPage<boolean>(
    cdp,
    `(() => {
      const button = [...document.querySelectorAll('button')].find((candidate) => {
        const label = candidate.getAttribute('aria-label') || candidate.getAttribute('title') || '';
        return /Settings|Настройки/i.test(label);
      });
      if (!(button instanceof HTMLElement)) return false;
      button.click();
      return true;
    })()`,
  );
  if (!opened || !(await waitForPageCondition(cdp, `Boolean(document.querySelector('[role="dialog"] form'))`, 15_000))) {
    throw new Error('settings_dialog_not_open');
  }
}

async function completeOnboardingToCampaignHome(cdp: CdpClient): Promise<void> {
  if (!(await evaluateInPage<boolean>(cdp, `Boolean(document.querySelector('.dm-onboarding'))`))) {
    return;
  }
  if (
    !(await evaluateInPage<boolean>(cdp, `Boolean(document.querySelector('.dm-hero-cards'))`))
  ) {
    if (await clickFirst(cdp, '.dm-onboarding-actions .dm-onboarding-btn-primary')) {
      await waitForPageCondition(cdp, `document.querySelectorAll('.dm-preset-card').length >= 5`, 15_000);
    }
    if (
      await evaluateInPage<boolean>(
        cdp,
        `document.querySelectorAll('.dm-preset-card').length >= 5`,
      )
    ) {
      await evaluateInPage(cdp, `document.querySelectorAll('.dm-preset-card')[4]?.click()`);
      if (!(await clickFirst(cdp, '.dm-onboarding-actions .dm-onboarding-btn-primary'))) {
        throw new Error('onboarding_manual_continue_missing');
      }
    }
    if (
      !(await waitForPageCondition(cdp, `Boolean(document.querySelector('.dm-hero-cards'))`, 15_000))
    ) {
      throw new Error('onboarding_hero_step_missing');
    }
  }
  const selectedPresetHero = await clickFirst(cdp, '.dm-hero-card');
  if (
    !selectedPresetHero ||
    !(await waitForPageCondition(cdp, `!document.querySelector('.dm-onboarding')`, 20_000))
  ) {
    throw new Error('onboarding_not_completed');
  }
  if (
    !(await waitForPageCondition(
      cdp,
      `[...document.querySelectorAll('button')].some((button) =>
        /New campaign|РќРѕРІР°СЏ РєР°РјРїР°РЅРёСЏ/i.test((button.textContent || '').trim())
      )`,
      20_000,
    ))
  ) {
    throw new Error('onboarding_campaign_home_missing');
  }
}

async function setSelectWithOption(
  cdp: CdpClient,
  optionValue: string,
  required = true,
): Promise<boolean> {
  const alreadySelected = await evaluateInPage<boolean>(
    cdp,
    `Boolean(document.querySelector('[role="dialog"] [role="combobox"][data-value="' + CSS.escape(${JSON.stringify(optionValue)}) + '"]'))`,
  );
  if (alreadySelected) return true;

  const count = await evaluateInPage<number>(
    cdp,
    `document.querySelectorAll('[role="dialog"] [role="combobox"]').length`,
  );
  for (let index = 0; index < count; index += 1) {
    await evaluateInPage(
      cdp,
      `document.querySelectorAll('[role="dialog"] [role="combobox"]')[${index}]?.click()`,
    );
    await sleep(50);
    const changed = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const option = document.querySelector(
          '[role="dialog"] [role="option"][data-value="' + CSS.escape(${JSON.stringify(optionValue)}) + '"]'
        );
        if (!(option instanceof HTMLElement)) return false;
        option.click();
        return true;
      })()`,
    );
    if (changed) return true;
    await evaluateInPage(
      cdp,
      `(() => {
        const combobox = document.querySelectorAll('[role="dialog"] [role="combobox"]')[${index}];
        if (combobox?.getAttribute('aria-expanded') === 'true') combobox.click();
      })()`,
    );
  }
  if (required) throw new Error(`settings_option_missing:${optionValue}`);
  return false;
}

async function saveSettings(cdp: CdpClient): Promise<void> {
  if (!(await clickFirst(cdp, '[role="dialog"] button[type="submit"][form="settings-form"]'))) {
    throw new Error('settings_save_missing');
  }
  if (!(await waitForPageCondition(cdp, `!document.querySelector('[role="dialog"] form')`, 25_000))) {
    const code = await evaluateInPage<string>(
      cdp,
      `document.querySelector('[data-testid="settings-save-error"]') ? 'settings_save_rejected' : 'settings_close_timeout'`,
    );
    throw new Error(code);
  }
}

async function configureLocalRuntime(
  cdp: CdpClient,
  port: number,
  strategy: 'auto-swap' | 'disable-image-gen',
  startRuntime = true,
): Promise<Awaited<ReturnType<typeof runtimeStatus>>> {
  await openSettings(cdp);
  await setSelectWithOption(cdp, 'en', false);
  await setSelectWithOption(cdp, 'local-mistralrs');
  await sleep(500);
  await setSelectWithOption(cdp, strategy);
  let status = await runtimeStatus(port);
  if (startRuntime && status.llm?.state !== 'ready') {
    const started = await clickFirst(cdp, '[role="dialog"] button[data-status]');
    if (!started) throw new Error('runtime_start_control_missing');
    status = await waitForLlmReady(
      port,
      resolveRuntimeTimeoutMs(process.env.LOCAL_RUNTIME_TIMEOUT_MS),
    );
    await sleep(6_000);
  }
  await saveSettings(cdp);
  return status;
}

async function campaignBootstrapAction(cdp: CdpClient) {
  const surface = await evaluateInPage<{
    wizardVisible: boolean;
    campaignCreateVisible: boolean;
    newCampaignAvailable: boolean;
    gameTableVisible: boolean;
  }>(
    cdp,
    `(() => {
      const wizard = document.querySelector('.dm-wizard');
      return {
        wizardVisible: wizard instanceof HTMLElement && getComputedStyle(wizard).display !== 'none',
        campaignCreateVisible: Boolean(document.querySelector('main ol li[data-active="true"]')),
        newCampaignAvailable: [...document.querySelectorAll('button')].some((button) =>
          /New campaign|Новая кампания/i.test((button.textContent || '').trim())
        ),
        gameTableVisible: Boolean(
          document.querySelector('.dm-vtt-panel') && document.querySelector('textarea')
        ),
      };
    })()`,
  );
  return resolveCampaignBootstrapAction(surface);
}

async function completeCampaignCreation(cdp: CdpClient): Promise<void> {
  if (
    !(await waitForPageCondition(
      cdp,
      `Boolean(document.querySelector('main ol li[data-active="true"]'))`,
      15_000,
    ))
  ) {
    throw new Error('campaign_create_not_visible');
  }

  const currentStep = () =>
    evaluateInPage<number>(
      cdp,
      `Number(document.querySelector('main ol li[data-active="true"] span')?.textContent || 0)`,
    );
  let step = await currentStep();
  if (step === 1) {
    if (!(await clickFirst(cdp, 'main footer button:last-child:not(:disabled)'))) {
      throw new Error('campaign_source_continue_missing');
    }
    if (
      !(await waitForPageCondition(
        cdp,
        `Number(document.querySelector('main ol li[data-active="true"] span')?.textContent || 0) === 2`,
        15_000,
      ))
    ) {
      throw new Error('campaign_tuning_not_visible');
    }
    step = 2;
  }
  if (step === 2) {
    const titleFilled = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const title = document.querySelector('main input:not([type="radio"])');
        if (!(title instanceof HTMLInputElement)) return false;
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
        setter?.call(title, ${JSON.stringify(`CDP Campaign ${runId}`)});
        title.dispatchEvent(new Event('input', { bubbles: true }));
        return true;
      })()`,
    );
    const premiseFilled = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const premise = document.querySelector('main textarea');
        if (!(premise instanceof HTMLTextAreaElement)) return false;
        const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set;
        setter?.call(premise, 'A moonlit gate guards the road into an old frontier realm.');
        premise.dispatchEvent(new Event('input', { bubbles: true }));
        return true;
      })()`,
    );
    if (!titleFilled || !premiseFilled) throw new Error('campaign_tuning_fields_missing');
    if (
      !(await waitForPageCondition(
        cdp,
        `Boolean(document.querySelector('main footer button:last-child:not(:disabled)'))`,
        5_000,
      ))
    ) {
      throw new Error('campaign_tuning_invalid');
    }
    if (!(await clickFirst(cdp, 'main footer button:last-child:not(:disabled)'))) {
      throw new Error('campaign_tuning_continue_missing');
    }
    if (
      !(await waitForPageCondition(
        cdp,
        `Number(document.querySelector('main ol li[data-active="true"] span')?.textContent || 0) === 3`,
        15_000,
      ))
    ) {
      throw new Error('campaign_character_handoff_missing');
    }
    step = 3;
  }
  for (const nextStep of [4]) {
    if (!(await clickFirst(cdp, 'main footer button:last-child:not(:disabled)'))) {
      throw new Error('campaign_preparation_handoff_missing');
    }
    if (
      !(await waitForPageCondition(
        cdp,
        `Number(document.querySelector('main ol li[data-active="true"] span')?.textContent || 0) === ${nextStep}`,
        15_000,
      ))
    ) {
      throw new Error('campaign_preparation_handoff_failed');
    }
    step = nextStep;
  }
  if (step !== 4 || !(await clickFirst(cdp, 'main footer button:last-child:not(:disabled)'))) {
    throw new Error('campaign_create_submit_missing');
  }
  if (
    !(await waitForPageCondition(
      cdp,
      `(() => { const wizard=document.querySelector('.dm-wizard'); return wizard instanceof HTMLElement && getComputedStyle(wizard).display !== 'none'; })()`,
      30_000,
    ))
  ) {
    throw new Error('campaign_character_handoff_failed');
  }
}

async function createCharacter(cdp: CdpClient): Promise<string> {
  let bootstrapAction = await campaignBootstrapAction(cdp);
  if (bootstrapAction === 'game_ready') {
    const heroVisible = await evaluateInPage<boolean>(
      cdp,
      `Boolean([...document.querySelectorAll('button')].find((button) =>
        (button.getAttribute('aria-label') || '').includes(${JSON.stringify(heroName)})))`,
    );
    if (!heroVisible) throw new Error('prepared_character_not_visible');
    return `pc-${runId}`;
  }
  if (bootstrapAction === 'open_campaign_create') {
    const opened = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const button = [...document.querySelectorAll('button')].find((candidate) =>
          /New campaign|Новая кампания/i.test((candidate.textContent || '').trim())
        );
        if (!(button instanceof HTMLElement)) return false;
        button.click();
        return true;
      })()`,
    );
    if (!opened) throw new Error('campaign_create_control_missing');
    bootstrapAction = 'complete_campaign_create';
  }
  if (bootstrapAction === 'complete_campaign_create') {
    await completeCampaignCreation(cdp);
  } else if (bootstrapAction === 'legacy_character_entry') {
    await openSettings(cdp);
    const recreate = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const button = [...document.querySelectorAll('[role="dialog"] button')]
          .find((candidate) => /Re-create character|Пересоздать персонажа/i.test(candidate.textContent || ''));
        if (!(button instanceof HTMLElement)) return false;
        button.click();
        return true;
      })()`,
    );
    if (!recreate) throw new Error('character_wizard_not_open');
  }
  if (!(await waitForPageCondition(cdp, `(() => { const w=document.querySelector('.dm-wizard'); return w instanceof HTMLElement && getComputedStyle(w).display !== 'none'; })()`, 30_000))) {
    throw new Error('character_wizard_not_visible');
  }

  for (const index of [0, 1, 2]) {
    await evaluateInPage(
      cdp,
      `document.querySelectorAll('.dm-wizard [role="tab"]')[${index}]?.click()`,
    );
    if (
      !(await waitForPageCondition(
        cdp,
        `Boolean(document.querySelector('.dm-wizard-panel [role="radio"]'))`,
        30_000,
      ))
    ) {
      throw new Error(`character_choice_not_loaded:${index}`);
    }
    const selected = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const choice = document.querySelector('.dm-wizard-panel [role="radio"]');
        if (!(choice instanceof HTMLElement)) return false;
        choice.click();
        return true;
      })()`,
    );
    if (!selected) throw new Error(`character_required_choice_missing:${index}`);
  }
  await evaluateInPage(cdp, `document.querySelectorAll('.dm-wizard [role="tab"]')[3]?.click()`);
  if (!(await waitForPageCondition(cdp, `document.querySelectorAll('.dm-wizard-panel [role="radio"]').length >= 2`, 15_000))) {
    throw new Error('character_abilities_not_loaded');
  }
  const abilities = await evaluateInPage<boolean>(
    cdp,
    `(() => { const choice = document.querySelectorAll('.dm-wizard-panel [role="radio"]')[1]; if (!(choice instanceof HTMLElement)) return false; choice.click(); return true; })()`,
  );
  if (!abilities) throw new Error('character_abilities_missing');

  await evaluateInPage(cdp, `document.querySelectorAll('.dm-wizard [role="tab"]')[6]?.click()`);
  if (!(await waitForPageCondition(cdp, `Boolean(document.querySelector('.dm-wizard-panel [role="radio"]'))`, 15_000))) {
    throw new Error('character_equipment_not_loaded');
  }
  if (!(await clickFirst(cdp, '.dm-wizard-panel [role="radio"]'))) {
    throw new Error('character_equipment_missing');
  }

  await evaluateInPage(cdp, `document.querySelectorAll('.dm-wizard [role="tab"]')[7]?.click()`);
  if (!(await waitForPageCondition(cdp, `Boolean(document.querySelector('#dm-persona-name'))`, 15_000))) {
    throw new Error('character_persona_not_loaded');
  }
  const nameFocused = await evaluateInPage<boolean>(
    cdp,
    `(() => { const input = document.querySelector('#dm-persona-name'); if (!(input instanceof HTMLInputElement)) return false; input.focus(); input.select(); return true; })()`,
  );
  if (!nameFocused) throw new Error('character_name_missing');
  await cdp.send('Input.insertText', { text: heroName });
  if (
    !(await waitForPageCondition(
      cdp,
      `document.querySelector('#dm-persona-name')?.value.endsWith(${JSON.stringify(heroName)})`,
      5_000,
    ))
  ) {
    throw new Error('character_name_not_committed');
  }

  await evaluateInPage(cdp, `document.querySelectorAll('.dm-wizard [role="tab"]')[9]?.click()`);
  if (!(await waitForPageCondition(cdp, `Boolean(document.querySelector('.dm-wizard-action-bar button:not(:disabled)'))`, 15_000))) {
    throw new Error('character_review_blocked');
  }
  const submitted = await clickFirst(cdp, '.dm-wizard-action-bar button:not(:disabled)');
  if (!submitted) throw new Error('character_review_blocked');
  await sleep(300);
  await evaluateInPage(
    cdp,
    `(() => {
      const dialogs = [...document.querySelectorAll('[role="dialog"]')];
      const confirm = dialogs.flatMap((dialog) => [...dialog.querySelectorAll('button')])
        .find((button) => /^(Confirm|Подтвердить)$/i.test((button.textContent || '').trim()));
      if (confirm instanceof HTMLElement) confirm.click();
      return true;
    })()`,
  );
  if (!(await waitForPageCondition(cdp, `!document.querySelector('.dm-wizard')`, 30_000))) {
    throw new Error('character_wizard_not_closed');
  }
  if (
    !(await waitForPageCondition(
      cdp,
      `Boolean([...document.querySelectorAll('button')].find((button) =>
        (button.getAttribute('aria-label') || '').includes(${JSON.stringify(heroName)})))`,
      15_000,
    ))
  ) {
    throw new Error('created_character_not_visible');
  }
  return `pc-${runId}`;
}

async function submitChatAndWait(
  cdp: CdpClient,
  message: string,
): Promise<void> {
  const baseline = await evaluateInPage<number>(
    cdp,
    `document.querySelectorAll('[data-testid="bubble"][data-role="assistant"]').length`,
  );
  const baselineSuccessfulToolResults = await evaluateInPage<number>(
    cdp,
    `document.querySelectorAll('[data-tool-name][data-status="success"]').length`,
  );
  const submitted = await evaluateInPage<boolean>(
    cdp,
    `(() => {
      const textarea = document.querySelector('textarea');
      if (!(textarea instanceof HTMLTextAreaElement)) return false;
      const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set;
      setter?.call(textarea, ${JSON.stringify(message)});
      textarea.dispatchEvent(new Event('input', { bubbles: true }));
      textarea.focus();
      textarea.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Enter', code: 'Enter', bubbles: true, cancelable: true
      }));
      return true;
    })()`,
  );
  if (!submitted) throw new Error('chat_composer_missing');

  const deadline = Date.now() + resolveDmResponseTimeoutMs(process.env.DM_RESPONSE_TIMEOUT_MS);
  while (Date.now() < deadline) {
    const snapshot = await evaluateInPage<DmReplySnapshot>(
      cdp,
      `(() => {
        const history = document.querySelector('[data-testid="chat-history"]');
        const assistantMessages = [...document.querySelectorAll(
          '[data-testid="bubble"][data-role="assistant"]'
        )].map((bubble) => (bubble.textContent || '').trim());
        const error = document.querySelector('[data-testid="chat-error"]');
        return {
          assistantMessages,
          successfulToolResults: document.querySelectorAll(
            '[data-tool-name][data-status="success"]'
          ).length,
          isStreaming: history?.getAttribute('data-streaming') === 'true',
          errorCode: error?.getAttribute('data-error-code') || undefined,
        };
      })()`,
    );
    const state = evaluateDmReply(snapshot, baseline, baselineSuccessfulToolResults);
    if (state.status === 'error') throw new Error(`agent_turn_${state.code}`);
    if (state.status === 'complete') return;
    await sleep(1_000);
  }
  throw new Error('agent_turn_timeout');
}

type SafeTurnEvent = {
  event: string;
  toolName?: string;
  isError?: boolean;
  kind?: string;
  source?: string;
  assetId?: string;
  projection?: {
    encounterId?: string;
    revision?: number;
    active?: boolean;
    currentCombatant?: string;
    combatants?: { id?: string; x?: number; y?: number }[];
  };
  npcId?: string;
  journalId?: string;
};

function safeToolResult(toolName: string, result: Record<string, unknown>): SafeTurnEvent {
    const rawProjection = result.projection && typeof result.projection === 'object'
      ? result.projection as Record<string, unknown>
      : undefined;
    const snapshot = rawProjection?.snapshot && typeof rawProjection.snapshot === 'object'
      ? rawProjection.snapshot as Record<string, unknown>
      : undefined;
    const combatants = Array.isArray(snapshot?.combatants)
      ? snapshot.combatants.flatMap((combatant) => {
          if (!combatant || typeof combatant !== 'object' || Array.isArray(combatant)) return [];
          const value = combatant as Record<string, unknown>;
          const position = value.position && typeof value.position === 'object'
            ? value.position as Record<string, unknown>
            : {};
          return [{
            id: typeof value.id === 'string' ? value.id : undefined,
            x: typeof position.x === 'number' ? position.x : undefined,
            y: typeof position.y === 'number' ? position.y : undefined,
          }];
        })
      : [];
    return {
      event: 'tool_call_result',
      toolName,
      isError: false,
      ...(rawProjection ? {
        projection: {
          encounterId: typeof rawProjection.encounter_id === 'string' ? rawProjection.encounter_id : undefined,
          revision: typeof rawProjection.revision === 'number' ? rawProjection.revision : undefined,
          active: typeof snapshot?.active === 'boolean' ? snapshot.active : undefined,
          currentCombatant: typeof snapshot?.current_combatant === 'string' ? snapshot.current_combatant : undefined,
          combatants,
        },
      } : {}),
      npcId: typeof result.npc_id === 'string' ? result.npc_id : undefined,
      journalId: typeof result.entry_id === 'string' ? result.entry_id : undefined,
    };
}

async function optionalRenderedToolResult(
  cdp: CdpClient,
  toolName: string,
): Promise<SafeTurnEvent | null> {
  const result = await evaluateInPage<Record<string, unknown> | null>(
    cdp,
    `(() => {
      const cards = [...document.querySelectorAll('[data-tool-name="${toolName}"][data-status="success"]')];
      const card = cards.at(-1);
      const blocks = card ? [...card.querySelectorAll('pre')] : [];
      const text = blocks.at(-1)?.textContent;
      if (!text) return null;
      try { return JSON.parse(text); } catch { return null; }
    })()`,
  );
  return result ? safeToolResult(toolName, result) : null;
}

async function renderedToolResult(cdp: CdpClient, toolName: string): Promise<SafeTurnEvent> {
  const event = await optionalRenderedToolResult(cdp, toolName);
  if (!event) throw new Error(`rendered_tool_result_missing:${toolName}`);
  return event;
}

async function waitForRenderedToolResult(
  cdp: CdpClient,
  toolName: string,
  timeoutMs = 20_000,
): Promise<SafeTurnEvent | null> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const event = await optionalRenderedToolResult(cdp, toolName);
    if (event) return event;
    await sleep(250);
  }
  return null;
}

async function renderedMedia(cdp: CdpClient, toolName: string): Promise<SafeTurnEvent> {
  const event = await evaluateInPage<SafeTurnEvent | null>(
    cdp,
    `(() => {
      const cards = [...document.querySelectorAll('[data-tool-name="${toolName}"][data-status="success"]')];
      const card = cards.at(-1);
      if (!(card instanceof HTMLElement)) return null;
      return { event: 'image_generated', kind: card.dataset.imageKind,
        source: card.dataset.imageSource, assetId: card.dataset.imageAssetId };
    })()`,
  );
  if (!event) throw new Error(`rendered_media_missing:${toolName}`);
  return event;
}

async function directImage(
  cdp: CdpClient,
  port: number,
  stylePreset: string,
): Promise<{ status: number; source?: 'generated' | 'bundled'; assetId?: string }> {
  return evaluateInPage(
    cdp,
    `(async () => {
      const response = await fetch('http://127.0.0.1:${port}/image/generate', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ prompt: 'classic fantasy acceptance scene ${runId}', style_preset: ${JSON.stringify(stylePreset)} }),
      });
      let body = {};
      try { body = await response.json(); } catch {}
      return {
        status: response.status,
        source: body.source,
        assetId: typeof body.asset_id === 'string' ? body.asset_id : undefined,
      };
    })()`,
    360_000,
  );
}

async function driveCombat(
  cdp: CdpClient,
  startEvent: SafeTurnEvent,
  evidence: CampaignEvidence[],
): Promise<void> {
  const start = startEvent.projection;
  if (!start?.encounterId || !Number.isInteger(start.revision)) {
    throw new Error('start_combat_projection_missing');
  }
  const startRevision = start.revision!;
  evidence.push({ checkpoint: 'combat_started', revision: startRevision });
  logCheckpoint(evidence.at(-1)!);

  const tokenBefore = await evaluateInPage<{ id: string; left: string; top: string } | null>(
    cdp,
    `(() => {
      const token = document.querySelector('[data-testid^="combat-token-"][data-active="true"]');
      if (!(token instanceof HTMLElement)) return null;
      return { id: token.dataset.testid.slice('combat-token-'.length), left: token.style.left, top: token.style.top };
    })()`,
  );
  if (!tokenBefore) throw new Error('active_combat_token_missing');
  const scrollSnapshot = await evaluateInPage<{
    viewportHeight: number;
    documentHeight: number;
    bodyHeight: number;
    chatPanelHeight: number;
    chatClientHeight: number;
    chatScrollHeight: number;
    bodyOverflow: string;
    chatOverflowY: string;
  }>(
    cdp,
    `(() => {
      const chatPanel = document.querySelector('.dm-chat-panel');
      const chat = document.querySelector('[data-testid="chat-history"]');
      if (!(chatPanel instanceof HTMLElement) || !(chat instanceof HTMLElement)) {
        throw new Error('chat_scroll_container_missing');
      }
      return {
        viewportHeight: window.innerHeight,
        documentHeight: document.documentElement.scrollHeight,
        bodyHeight: document.body.scrollHeight,
        chatPanelHeight: chatPanel.getBoundingClientRect().height,
        chatClientHeight: chat.clientHeight,
        chatScrollHeight: chat.scrollHeight,
        bodyOverflow: getComputedStyle(document.body).overflow,
        chatOverflowY: getComputedStyle(chat).overflowY,
      };
    })()`,
  );
  if (!isChatScrollContained(scrollSnapshot)) {
    throw new Error(`app_shell_overflow:${JSON.stringify(scrollSnapshot)}`);
  }
  const tokenCenter = await evaluateInPage<{ x: number; y: number } | null>(
    cdp,
    `(() => {
      const token = document.querySelector('[data-testid="combat-token-${tokenBefore.id}"]');
      if (!(token instanceof HTMLElement)) return null;
      const box = token.getBoundingClientRect();
      return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
    })()`,
  );
  if (!tokenCenter) throw new Error('combat_drag_target_missing');
  // WebView2 does not consistently grant pointer capture to mouse events
  // synthesized through CDP. Once the pointer leaves the 30 px token, the
  // release can land on the canvas and React never calls the token's
  // onPointerEnd handler. Dispatch the complete pointer sequence to the same
  // live token element so this acceptance step exercises the production drag
  // handler deterministically.
  const dragDispatched = await evaluateInPage<boolean>(
    cdp,
    `(() => {
      const token = document.querySelector('[data-testid="combat-token-${tokenBefore.id}"]');
      if (!(token instanceof HTMLElement) || typeof PointerEvent !== 'function') return false;
      const shared = {
        bubbles: true,
        cancelable: true,
        pointerId: 91,
        pointerType: 'mouse',
        isPrimary: true,
      };
      token.dispatchEvent(new PointerEvent('pointerdown', {
        ...shared,
        clientX: ${tokenCenter.x},
        clientY: ${tokenCenter.y},
        button: 0,
        buttons: 1,
      }));
      token.dispatchEvent(new PointerEvent('pointermove', {
        ...shared,
        clientX: ${tokenCenter.x + 60},
        clientY: ${tokenCenter.y},
        button: 0,
        buttons: 1,
      }));
      token.dispatchEvent(new PointerEvent('pointerup', {
        ...shared,
        clientX: ${tokenCenter.x + 60},
        clientY: ${tokenCenter.y},
        button: 0,
        buttons: 0,
      }));
      return true;
    })()`,
  );
  if (!dragDispatched) throw new Error('combat_drag_dispatch_failed');
  evidence.push({ checkpoint: 'move_requested', revision: startRevision });
  logCheckpoint(evidence.at(-1)!);
  if (
    !(await waitForPageCondition(
      cdp,
      `Number(document.querySelector('[data-testid="combat-overlay"]')?.dataset.revision) > ${startRevision}`,
      30_000,
    ))
  ) {
    throw new Error('combat_move_response_missing');
  }
  const moveRevision = await evaluateInPage<number>(
    cdp,
    `Number(document.querySelector('[data-testid="combat-overlay"]')?.dataset.revision)`,
  );
  if (!Number.isInteger(moveRevision) || moveRevision <= startRevision) {
    throw new Error('combat_move_revision_invalid');
  }
  const tokenAfter = await evaluateInPage<{ left: string; top: string } | null>(
    cdp,
    `(() => {
      const token = document.querySelector('[data-testid="combat-token-${tokenBefore.id}"]');
      return token instanceof HTMLElement ? { left: token.style.left, top: token.style.top } : null;
    })()`,
  );
  if (!tokenAfter || (tokenAfter.left === tokenBefore.left && tokenAfter.top === tokenBefore.top)) {
    throw new Error('combat_move_not_reconciled');
  }
  evidence.push({ checkpoint: 'combat_revision_advanced', revision: moveRevision });
  logCheckpoint(evidence.at(-1)!);

  if (!(await clickFirst(cdp, '[data-testid="action-btn-cast"]'))) {
    throw new Error('combat_cast_control_missing');
  }
  if (
    !(await waitForPageCondition(
      cdp,
      `Number(document.querySelector('[data-testid="combat-overlay"]')?.dataset.revision) > ${moveRevision}`,
      30_000,
    ))
  ) {
    throw new Error('combat_cast_response_missing');
  }
  const castRevision = await evaluateInPage<number>(
    cdp,
    `Number(document.querySelector('[data-testid="combat-overlay"]')?.dataset.revision)`,
  );
  if (!Number.isInteger(castRevision) || castRevision <= moveRevision) {
    throw new Error('combat_cast_revision_invalid');
  }
  evidence.push({ checkpoint: 'combat_acted', revision: castRevision });
  logCheckpoint(evidence.at(-1)!);

  const turnBefore = await evaluateInPage<string | null>(
    cdp,
    `document.querySelector('[data-testid="combat-overlay"]')?.dataset.currentTurn || null`,
  );
  if (!(await clickFirst(cdp, '[data-testid="action-btn-end_turn"]'))) {
    throw new Error('combat_end_turn_control_missing');
  }
  if (
    !(await waitForPageCondition(
      cdp,
      `(() => { const overlay=document.querySelector('[data-testid="combat-overlay"]'); return Number(overlay?.dataset.revision) > ${castRevision} && overlay?.dataset.currentTurn !== ${JSON.stringify(turnBefore)}; })()`,
      30_000,
    ))
  ) {
    throw new Error('combat_end_turn_response_missing');
  }
  const turnRevision = await evaluateInPage<number>(
    cdp,
    `Number(document.querySelector('[data-testid="combat-overlay"]')?.dataset.revision)`,
  );
  if (!Number.isInteger(turnRevision) || turnRevision <= castRevision) {
    throw new Error('combat_end_turn_revision_invalid');
  }
  evidence.push({ checkpoint: 'combat_turn_advanced', revision: turnRevision });
  logCheckpoint(evidence.at(-1)!);
}

async function observeNpc(cdp: CdpClient): Promise<string> {
  const openByLabel = async (pattern: string) =>
    evaluateInPage<boolean>(
      cdp,
      `(() => {
        const pattern = new RegExp(${JSON.stringify(pattern)}, 'i');
        const button = [...document.querySelectorAll('button')].find((candidate) =>
          pattern.test(candidate.getAttribute('aria-label') || candidate.textContent || '')
        );
        if (!(button instanceof HTMLElement)) return false;
        button.click();
        return true;
      })()`,
    );
  if (!(await openByLabel('NPCs|NPC|Персонаж'))) throw new Error('npc_view_control_missing');
  if (!(await waitForPageCondition(cdp, `Boolean(document.querySelector('[role="dialog"] article'))`, 15_000))) {
    throw new Error('npc_projection_missing');
  }
  const npcId = await evaluateInPage<string>(
    cdp,
    `document.querySelector('[role="dialog"] article h3')?.textContent?.trim() || 'observed-npc'`,
  );
  await evaluateInPage(
    cdp,
    `document.querySelector('[role="dialog"] button[aria-label]')?.click()`,
  );
  return npcId;
}

async function observeJournal(cdp: CdpClient): Promise<string | null> {
  const openByLabel = async (pattern: string) =>
    evaluateInPage<boolean>(
      cdp,
      `(() => {
        const pattern = new RegExp(${JSON.stringify(pattern)}, 'i');
        const button = [...document.querySelectorAll('button')].find((candidate) =>
          pattern.test(candidate.getAttribute('aria-label') || candidate.textContent || '')
        );
        if (!(button instanceof HTMLElement)) return false;
        button.click();
        return true;
      })()`,
    );
  if (!(await openByLabel('Journal|Журнал'))) throw new Error('journal_view_control_missing');
  await sleep(700);
  const entryCount = await evaluateInPage<number>(
    cdp,
    `document.querySelectorAll('[role="dialog"] article').length`,
  );
  await evaluateInPage(
    cdp,
    `document.querySelector('[role="dialog"] button[aria-label]')?.click()`,
  );
  return resolveJournalEvidenceId(entryCount);
}

async function quickSaveAndRestore(
  cdp: CdpClient,
): Promise<string> {
  const existingIds = await evaluateInPage<string[]>(
    cdp,
    `[...document.querySelectorAll('[data-save-id]')].map((node) => node.getAttribute('data-save-id')).filter(Boolean)`,
  );
  await evaluateInPage(
    cdp,
    `(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', code: 'KeyS', ctrlKey: true, bubbles: true }));
      return true;
    })()`,
  );
  if (
    !(await waitForPageCondition(
      cdp,
      `Boolean(document.querySelector('.dm-saves-toast'))`,
      30_000,
    ))
  ) {
    throw new Error('quick_save_response_missing');
  }
  await evaluateInPage(
    cdp,
    `(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', code: 'KeyS', ctrlKey: true, shiftKey: true, bubbles: true }));
      return true;
    })()`,
  );
  if (!(await waitForPageCondition(cdp, `Boolean(document.querySelector('.dm-saves-overlay'))`, 20_000))) {
    throw new Error('saves_view_not_open');
  }
  if (!(await waitForPageCondition(cdp, `Boolean(document.querySelector('[data-save-id]'))`, 30_000))) {
    throw new Error('created_save_not_listed');
  }
  const saveId = await evaluateInPage<string | null>(
    cdp,
    `(() => [...document.querySelectorAll('[data-save-id]')]
      .map((node) => node.getAttribute('data-save-id'))
      .find((id) => id && !${JSON.stringify(existingIds)}.includes(id)) ??
      document.querySelector('[data-save-id]')?.getAttribute('data-save-id') ?? null)()`,
  );
  if (!saveId) throw new Error('quick_save_failed:missing');
  await clickFirst(cdp, `[data-save-id="${saveId}"]`);
  const loadClicked = await evaluateInPage<boolean>(
    cdp,
    `(() => {
      const buttons = [...document.querySelectorAll('.dm-save-detail-actions button')];
      const button = buttons[0];
      if (!(button instanceof HTMLElement)) return false;
      button.click();
      return true;
    })()`,
  );
  if (!loadClicked) throw new Error('save_load_control_missing');
  if (
    !(await waitForPageCondition(
      cdp,
      `!document.querySelector('.dm-saves-overlay')`,
      30_000,
    ))
  ) {
    throw new Error('save_restore_response_missing');
  }
  return saveId;
}

async function main(): Promise<void> {
  const started = Date.now();
  await mkdir(artifactRoot, { recursive: true });
  const screenshots: string[] = [];
  const evidence: CampaignEvidence[] = [];
  let modelId: string | null = null;
  let generatedImage: SafeCampaignEvidenceFile['generatedImage'] = {
    status: 'not_runtime_tested',
  };
  const cdp = new CdpClient(await findAppPageWebSocket({ timeoutMs: 120_000 }));
  try {
    await cdp.open();
    await cdp.send('Runtime.enable');
    await cdp.send('Page.enable');
    if (!(await waitForPageCondition(cdp, `document.body.innerText.includes('DUNGEON MASTER AI')`, 180_000))) {
      throw new Error('application_not_rendered');
    }
    if (!(await evaluateInPage<boolean>(cdp, `Boolean(window.__TAURI_INTERNALS__)`))) {
      throw new Error('tauri_runtime_missing');
    }
    evidence.push({ checkpoint: 'tauri_present' });
    logCheckpoint(evidence.at(-1)!);

    const port = await backendPort(cdp);
    const health = await fetch(`http://127.0.0.1:${port}/health`);
    if (!health.ok) throw new Error(`backend_health_http_${health.status}`);
    evidence.push({ checkpoint: 'backend_ready', port });
    logCheckpoint(evidence.at(-1)!);
    await completeOnboardingToCampaignHome(cdp);

    const characterId = await createCharacter(cdp);
    evidence.push({ checkpoint: 'character_created', characterId });
    logCheckpoint(evidence.at(-1)!);

    const runtime = await configureLocalRuntime(cdp, port, 'disable-image-gen');
    modelId = runtime.model_id ?? null;
    generatedImage = { status: 'not_runtime_tested', errorCode: 'media_runtime_disabled' };
    const illustration = await directImage(cdp, port, 'classic');
    const map = await directImage(cdp, port, 'map');
    if (
      illustration.status >= 400 ||
      illustration.source !== 'bundled' ||
      !illustration.assetId ||
      map.status >= 400 ||
      map.source !== 'bundled' ||
      !map.assetId ||
      illustration.assetId === map.assetId
    ) {
      throw new Error('bundled_direct_media_contract_failed');
    }

    await submitChatAndWait(cdp, campaignPrompt);
    evidence.push({ checkpoint: 'assistant_reply_completed' });
    logCheckpoint(evidence.at(-1)!);

    const illustrationEvent = await renderedMedia(cdp, 'generate_illustration');
    const mapEvent = await renderedMedia(cdp, 'generate_map');
    if (
      illustrationEvent?.source !== 'bundled' ||
      !illustrationEvent.assetId ||
      mapEvent?.source !== 'bundled' ||
      !mapEvent.assetId
    ) {
      throw new Error('bundled_agent_media_missing');
    }
    evidence.push({
      checkpoint: 'bundled_illustration',
      source: 'bundled',
      assetId: illustrationEvent.assetId,
    });
    logCheckpoint(evidence.at(-1)!);
    evidence.push({ checkpoint: 'bundled_map', source: 'bundled', assetId: mapEvent.assetId });
    logCheckpoint(evidence.at(-1)!);
    if (
      !(await waitForPageCondition(
        cdp,
        `Boolean(document.querySelector('[data-testid="dm-vtt-map-bg"]'))`,
        30_000,
      ))
    ) {
      throw new Error('vtt_map_not_visible');
    }
    evidence.push({ checkpoint: 'vtt_visible' });
    logCheckpoint(evidence.at(-1)!);

    let startEvent = await waitForRenderedToolResult(cdp, 'start_combat', 10_000);
    if (!startEvent) {
      await submitChatAndWait(cdp, startCombatRetryPrompt);
      startEvent = await waitForRenderedToolResult(cdp, 'start_combat');
    }
    if (!startEvent) throw new Error('rendered_tool_result_missing:start_combat');
    const combatScene = await evaluateInPage<CombatSceneVisibilitySnapshot>(
      cdp,
      `(() => {
        const viewport = document.querySelector('[data-testid="dm-vtt"]');
        const map = document.querySelector('[data-testid="dm-vtt-map-bg"]');
        const overlay = document.querySelector('[data-testid="combat-overlay"]');
        if (!(viewport instanceof HTMLElement) || !(map instanceof HTMLElement) ||
            !(overlay instanceof HTMLElement)) throw new Error('combat_scene_missing');
        const rect = (element) => {
          const box = element.getBoundingClientRect();
          return { x: box.x, y: box.y, width: box.width, height: box.height };
        };
        const tokens = [...overlay.children]
          .filter((element) => element instanceof HTMLElement &&
            (element.dataset.testid || '').startsWith('combat-token-'))
          .map(rect);
        return { viewport: rect(viewport), map: rect(map), tokens };
      })()`,
    );
    if (!isCombatSceneVisible(combatScene)) throw new Error('combat_tokens_not_visible');
    const combatScreenshot = `${artifactRoot}/campaign-combat.png`;
    await captureScreenshot(cdp, combatScreenshot);
    screenshots.push(combatScreenshot);
    await driveCombat(cdp, startEvent, evidence);

    await submitChatAndWait(cdp, endCombatPrompt);
    let ended = await waitForRenderedToolResult(cdp, 'end_combat');
    if (!ended) {
      await submitChatAndWait(cdp, endCombatRetryPrompt);
      ended = await waitForRenderedToolResult(cdp, 'end_combat');
    }
    if (!ended?.projection || ended.projection.active !== false || !Number.isInteger(ended.projection.revision)) {
      throw new Error('authoritative_combat_end_missing');
    }
    if (
      !(await waitForPageCondition(
        cdp,
        `!document.querySelector('[data-testid="action-btn-end_turn"]')`,
        20_000,
      ))
    ) {
      throw new Error('combat_ui_not_ended');
    }
    evidence.push({ checkpoint: 'combat_ended', revision: ended.projection.revision! });
    logCheckpoint(evidence.at(-1)!);

    const npcId = await observeNpc(cdp);
    evidence.push({ checkpoint: 'npc_observed', npcId });
    logCheckpoint(evidence.at(-1)!);
    let journalId = await observeJournal(cdp);
    if (!journalId) {
      await submitChatAndWait(cdp, journalPrompt);
      journalId = await observeJournal(cdp);
    }
    if (!journalId) throw new Error('journal_projection_missing');
    evidence.push({ checkpoint: 'journal_observed', journalId });
    logCheckpoint(evidence.at(-1)!);

    const saveId = await quickSaveAndRestore(cdp);
    evidence.push({ checkpoint: 'save_created', saveId });
    logCheckpoint(evidence.at(-1)!);
    evidence.push({ checkpoint: 'save_restored', saveId });
    logCheckpoint(evidence.at(-1)!);

    const screenshot = `${artifactRoot}/campaign-complete.png`;
    await captureScreenshot(cdp, screenshot);
    screenshots.push(screenshot);
    const result = evaluateCampaignEvidence(evidence);
    if (!result.ok) throw new Error(`campaign_evidence_${result.code}`);
    const finished = Date.now();
    const output: SafeCampaignEvidenceFile = {
      schemaVersion: 1,
      status: 'passed',
      startedAt: new Date(started).toISOString(),
      finishedAt: new Date(finished).toISOString(),
      durationMs: finished - started,
      modelId,
      generatedImage,
      checkpoints: evidence,
      result,
      screenshots,
    };
    await writeFile(evidencePath, `${JSON.stringify(output, null, 2)}\n`, 'utf8');
    console.log(`PASS campaign_complete durationMs=${output.durationMs} evidence=${evidencePath}`);
  } catch (error) {
    const finished = Date.now();
    const result = evaluateCampaignEvidence(evidence);
    const output: SafeCampaignEvidenceFile = {
      schemaVersion: 1,
      status: 'failed',
      startedAt: new Date(started).toISOString(),
      finishedAt: new Date(finished).toISOString(),
      durationMs: finished - started,
      modelId,
      generatedImage,
      checkpoints: evidence,
      result,
      screenshots,
    };
    await writeFile(evidencePath, `${JSON.stringify(output, null, 2)}\n`, 'utf8');
    throw error;
  } finally {
    cdp.close();
  }
}

main().catch((error) => {
  console.error(`FAIL campaign code=${(error as Error).message} evidence=${evidencePath}`);
  process.exit(1);
});
