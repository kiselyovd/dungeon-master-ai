/**
 * Full real-Tauri play flow over raw CDP.
 *
 * The script switches the UI to the local provider, starts the runtime, saves
 * the settings, sends a player action and waits for a real DM response.
 * Backend port discovery is performed through the Tauri command instead of a
 * machine-specific temporary file.
 */
import {
  CdpClient,
  captureScreenshot,
  defaultArtifactPath,
  evaluateInPage,
  findAppPageWebSocket,
  waitForPageCondition,
} from './lib/cdp';
import { evaluateDmReply, type DmReplySnapshot } from './lib/dm-reply';

const prompt =
  process.env.PLAY_PROMPT ??
  'I push open the tavern door and look around the common room. Describe what I see.';
const screenshotPath =
  process.env.TAURI_PLAY_SCREENSHOT ?? defaultArtifactPath('tauri-live-play.png');

const sleep = (milliseconds: number) =>
  new Promise<void>((resolvePromise) => setTimeout(resolvePromise, milliseconds));
const log = (message: string): void => console.log(message);

async function clickButtonInDialog(cdp: CdpClient, patternSource: string): Promise<boolean> {
  return evaluateInPage<boolean>(
    cdp,
    `(() => {
      const pattern = new RegExp(${JSON.stringify(patternSource)}, 'i');
      const button = [...document.querySelectorAll('[role="dialog"] button')]
        .find((candidate) => pattern.test((candidate.textContent || '').trim()));
      if (button instanceof HTMLElement) button.click();
      return Boolean(button);
    })()`,
  );
}

async function backendPort(cdp: CdpClient): Promise<number> {
  const port = await evaluateInPage<number | null>(
    cdp,
    `(async () => {
      const invoke = window.__TAURI_INTERNALS__?.invoke;
      if (typeof invoke !== 'function') return null;
      try { return await invoke('backend_port'); } catch { return null; }
    })()`,
  );
  if (typeof port !== 'number' || port <= 0) {
    throw new Error(`dmai-server port is not available: ${String(port)}`);
  }
  return port;
}

async function runtimeReady(port: number): Promise<boolean> {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/local/runtime/status`);
    if (!response.ok) return false;
    const status = (await response.json()) as {
      llm?: { state?: string };
      image?: { state?: string };
    };
    return status.llm?.state === 'ready';
  } catch {
    return false;
  }
}

async function main(): Promise<void> {
  const cdp = new CdpClient(await findAppPageWebSocket({ timeoutMs: 90_000 }));
  try {
    await cdp.open();
    await cdp.send('Runtime.enable');
    await cdp.send('Page.enable');

    if (
      !(await waitForPageCondition(
        cdp,
        `document.body.innerText.includes('DUNGEON MASTER AI')`,
        180_000,
      ))
    ) {
      throw new Error('application did not render');
    }
    log('Application rendered in the real WebView.');

    const port = await backendPort(cdp);
    log(`dmai-server port: ${port}`);

    await evaluateInPage(
      cdp,
      `(() => {
        if (window.__dmaiCaptureInstalled) return true;
        window.__dmaiCaptureInstalled = true;
        window.__dmaiSettingsResult = null;
        window.__dmaiAgentResult = null;
        const originalFetch = window.fetch;
        window.fetch = async (...args) => {
          const url = String(args[0]);
          const response = await originalFetch(...args);
          if (url.includes('/settings/v2')) window.__dmaiSettingsResult = { status: response.status };
          if (url.includes('/agent/turn')) window.__dmaiAgentResult = { status: response.status };
          return response;
        };
        return true;
      })()`,
    );

    const settingsOpened = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const button = [...document.querySelectorAll('button')].find((candidate) =>
          /Настройк|Settings/i.test(candidate.getAttribute('aria-label') || candidate.textContent || '')
        );
        if (button instanceof HTMLElement) button.click();
        return Boolean(button);
      })()`,
    );
    if (!settingsOpened) throw new Error('Settings button was not found');

    if (!(await waitForPageCondition(cdp, `document.querySelector('[role="dialog"] select')`, 15_000))) {
      throw new Error('Settings dialog did not open');
    }

    const providerChanged = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const select = [...document.querySelectorAll('[role="dialog"] select')].find((candidate) =>
          [...candidate.options].some((option) => option.value === 'local-mistralrs')
        );
        if (!(select instanceof HTMLSelectElement)) return false;
        const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set;
        setter?.call(select, 'local-mistralrs');
        select.dispatchEvent(new Event('change', { bubbles: true }));
        return true;
      })()`,
    );
    if (!providerChanged) throw new Error('local-mistralrs provider option was not found');
    log('Provider switched to local-mistralrs.');

    const startClicked = await clickButtonInDialog(
      cdp,
      '^(Start runtimes|Start|Запустить рантаймы|Запустить)$',
    );
    log(`Start runtime clicked: ${startClicked}`);

    let ready = false;
    const runtimeDeadline = Date.now() + Number(process.env.LOCAL_RUNTIME_TIMEOUT_MS ?? 300_000);
    while (Date.now() < runtimeDeadline) {
      if (await runtimeReady(port)) {
        ready = true;
        break;
      }
      await sleep(5_000);
    }
    if (!ready) throw new Error('Local LLM runtime did not become ready before timeout');
    log('Local LLM runtime is ready.');

    await sleep(6_000);
    if (!(await clickButtonInDialog(cdp, '^(Сохранить|Save)$'))) {
      throw new Error('Save button was not found');
    }

    if (!(await waitForPageCondition(cdp, `!document.querySelector('[role="dialog"]')`, 20_000))) {
      const banner = await evaluateInPage<string>(
        cdp,
        `(document.querySelector('[data-testid="settings-save-error"]')?.textContent || '').slice(0, 300)`,
      );
      throw new Error(`Settings dialog did not close. ${banner}`);
    }

    const settingsResult = await evaluateInPage<{ status: number } | null>(
      cdp,
      'window.__dmaiSettingsResult',
    );
    if (!settingsResult || settingsResult.status >= 400) {
      throw new Error(`/settings/v2 failed: ${JSON.stringify(settingsResult)}`);
    }

    const baselineAssistantCount = await evaluateInPage<number>(
      cdp,
      `document.querySelectorAll('[data-testid="bubble"][data-role="assistant"]').length`,
    );

    const submitted = await evaluateInPage<boolean>(
      cdp,
      `(() => {
        const textarea = document.querySelector('textarea');
        if (!(textarea instanceof HTMLTextAreaElement)) return false;
        const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set;
        setter?.call(textarea, ${JSON.stringify(prompt)});
        textarea.dispatchEvent(new Event('input', { bubbles: true }));
        textarea.focus();
        textarea.dispatchEvent(new KeyboardEvent('keydown', {
          key: 'Enter', code: 'Enter', bubbles: true, cancelable: true
        }));
        return true;
      })()`,
    );
    if (!submitted) throw new Error('Chat textarea was not found');
    log(`Player action submitted; assistant baseline count: ${baselineAssistantCount}`);

    const responseDeadline = Date.now() + Number(process.env.DM_RESPONSE_TIMEOUT_MS ?? 300_000);
    let reply = '';
    while (Date.now() < responseDeadline) {
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
      const state = evaluateDmReply(snapshot, baselineAssistantCount);
      if (state.status === 'error') {
        throw new Error(`Dungeon Master turn failed: ${state.code}`);
      }
      if (state.status === 'complete') {
        reply = state.text;
        log(`Completed assistant response observed; assistant count: ${snapshot.assistantMessages.length}`);
        break;
      }
      await sleep(1_500);
    }

    await captureScreenshot(cdp, screenshotPath);
    log(`Screenshot: ${screenshotPath}`);

    const agentResult = await evaluateInPage<{ status: number } | null>(
      cdp,
      'window.__dmaiAgentResult',
    );
    if (!agentResult || agentResult.status >= 400) {
      throw new Error(`/agent/turn failed: ${JSON.stringify(agentResult)}`);
    }
    if (!reply) throw new Error('No Dungeon Master response was rendered');

    log('PASS live local-DM play flow');
  } finally {
    cdp.close();
  }
}

main().catch((error) => {
  console.error(`ERROR ${(error as Error).stack ?? String(error)}`);
  process.exit(1);
});
