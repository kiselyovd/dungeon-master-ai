/**
 * Real Tauri smoke test over the raw Chrome DevTools Protocol.
 *
 * Windows/WebView2 launch example:
 *   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9222'
 *   bun run tauri dev
 *   bun run e2e:tauri
 *
 * The test talks to the live Tauri WebView, the real dmai-server sidecar and
 * the real Tauri persistence plugins. It does not use the browser mock.
 */
import {
  CdpClient,
  captureScreenshot,
  defaultArtifactPath,
  evaluateInPage,
  findAppPageWebSocket,
  waitForPageCondition,
} from './lib/cdp';

const screenshotPath =
  process.env.TAURI_E2E_SCREENSHOT ?? defaultArtifactPath('tauri-real-e2e.png');

function log(message: string): void {
  console.log(message);
}

interface Check {
  name: string;
  ok: boolean;
  detail?: string;
}

async function main(): Promise<void> {
  const websocketUrl = await findAppPageWebSocket({ timeoutMs: 90_000 });
  log(`Connecting to the real Tauri WebView: ${websocketUrl}`);

  const cdp = new CdpClient(websocketUrl);
  const uncaughtExceptions: string[] = [];
  const checks: Check[] = [];
  const record = (name: string, ok: boolean, detail?: string) => {
    checks.push({ name, ok, detail });
    log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ` - ${detail}` : ''}`);
  };

  try {
    await cdp.open();
    await cdp.send('Runtime.enable');
    await cdp.send('Page.enable');
    cdp.on('Runtime.exceptionThrown', (params) => {
      const event = params as {
        exceptionDetails?: { text?: string; exception?: { description?: string } };
      };
      uncaughtExceptions.push(
        event.exceptionDetails?.exception?.description ??
          event.exceptionDetails?.text ??
          'Unknown uncaught page exception',
      );
    });

    const hasTauri = await evaluateInPage<boolean>(cdp, 'Boolean(window.__TAURI_INTERNALS__)');
    record('real Tauri runtime is present', hasTauri);

    const appRendered = await waitForPageCondition(
      cdp,
      `document.body.innerText.includes('DUNGEON MASTER AI') && document.body.children.length > 0`,
      180_000,
    );
    record('application shell renders in the real WebView', appRendered);

    const backendReady = await waitForPageCondition(
      cdp,
      `!document.querySelector('.dm-splash') && Boolean(document.querySelector('textarea'))`,
      120_000,
    );
    record('splash closes and chat composer mounts', backendReady);

    const backendPort = await evaluateInPage<number | null>(
      cdp,
      `(async () => {
        try {
          const invoke = window.__TAURI_INTERNALS__?.invoke;
          return typeof invoke === 'function' ? await invoke('backend_port') : null;
        } catch { return null; }
      })()`,
    );
    record(
      'backend sidecar reports a dynamic port',
      typeof backendPort === 'number' && backendPort > 0,
      backendPort === null ? 'no port returned' : String(backendPort),
    );

    const noOnboarding = await evaluateInPage<boolean>(
      cdp,
      `!(/Step\s+\d+\s+of\s+\d+/i.test(document.body.innerText))`,
    );
    record('persisted onboarding state does not regress', noOnboarding);

    const pageTitle = await evaluateInPage<string>(cdp, 'document.title');
    record('window title is configured', pageTitle.includes('Dungeon Master AI'), pageTitle);

    await captureScreenshot(cdp, screenshotPath);
    log(`Screenshot: ${screenshotPath}`);

    record(
      'no uncaught page exceptions during smoke test',
      uncaughtExceptions.length === 0,
      uncaughtExceptions.slice(0, 3).join(' | ') || undefined,
    );
  } finally {
    cdp.close();
  }

  const failures = checks.filter((check) => !check.ok);
  log(`\n=== ${checks.length - failures.length}/${checks.length} real-Tauri checks passed ===`);
  if (failures.length > 0) process.exit(1);
}

main().catch((error) => {
  console.error(`ERROR ${(error as Error).stack ?? String(error)}`);
  process.exit(1);
});
