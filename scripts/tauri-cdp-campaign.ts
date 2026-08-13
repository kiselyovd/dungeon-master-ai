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
  isManualHeroButtonLabel,
  type CampaignEvidence,
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
  'The acceptance encounter is complete. Call end_combat exactly once, then narrate one short sentence.';
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

async function completeOnboardingToWizard(cdp: CdpClient): Promise<void> {
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
  const opened = await evaluateInPage<boolean>(
    cdp,
    `(() => {
      const button = [...document.querySelectorAll('.dm-onboarding-actions button')]
        .find((candidate) => (${isManualHeroButtonLabel.toString()})(candidate.textContent || ''));
      if (!(button instanceof HTMLElement)) return false;
      button.click();
      return true;
    })()`,
  );
  if (!opened || !(await waitForPageCondition(cdp, `!document.querySelector('.dm-onboarding')`, 20_000))) {
    throw new Error('onboarding_not_completed');
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
      Number(process.env.LOCAL_RUNTIME_TIMEOUT_MS ?? 420_000),
    );
    await sleep(6_000);
  }
  await saveSettings(cdp);
  return status;
}

async function createCharacter(cdp: CdpClient): Promise<string> {
  const wizardVisible = await evaluateInPage<boolean>(
    cdp,
    `(() => { const wizard = document.querySelector('.dm-wizard'); return wizard instanceof HTMLElement && getComputedStyle(wizard).display !== 'none'; })()`,
  );
  if (!wizardVisible) {
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

  const deadline = Date.now() + Number(process.env.DM_RESPONSE_TIMEOUT_MS ?? 360_000);
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
          isStreaming: history?.getAttribute('data-streaming') === 'true',
          errorCode: error?.getAttribute('data-error-code') || undefined,
        };
      })()`,
    );
    const state = evaluateDmReply(snapshot, baseline);
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

async function renderedToolResult(cdp: CdpClient, toolName: string): Promise<SafeTurnEvent> {
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
  if (!result) throw new Error(`rendered_tool_result_missing:${toolName}`);
  return safeToolResult(toolName, result);
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
  await cdp.send('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    x: tokenCenter.x,
    y: tokenCenter.y,
    button: 'left',
    buttons: 1,
    clickCount: 1,
  });
  await cdp.send('Input.dispatchMouseEvent', {
    type: 'mouseMoved',
    x: tokenCenter.x + 60,
    y: tokenCenter.y,
    button: 'left',
    buttons: 1,
  });
  await cdp.send('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    x: tokenCenter.x + 60,
    y: tokenCenter.y,
    button: 'left',
    buttons: 0,
    clickCount: 1,
  });
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
  const journalId = await evaluateInPage<string | null>(
    cdp,
    `document.querySelector('[role="dialog"] article h3')?.textContent?.trim() || null`,
  );
  await evaluateInPage(
    cdp,
    `document.querySelector('[role="dialog"] button[aria-label]')?.click()`,
  );
  return journalId;
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
    await completeOnboardingToWizard(cdp);

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

    const startEvent = await renderedToolResult(cdp, 'start_combat');
    await driveCombat(cdp, startEvent, evidence);

    await submitChatAndWait(cdp, endCombatPrompt);
    const ended = await renderedToolResult(cdp, 'end_combat');
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
