/**
 * Real-Tauri e2e over the raw Chrome DevTools Protocol.
 *
 * Drives the LIVE WebView2 window (not the vite/browser mock) with the real
 * `dmai-server` sidecar running behind it. WebView2 exposes a page-level CDP
 * endpoint but not the browser-level endpoint Playwright's connectOverCDP
 * needs, so this talks CDP directly over bun's native WebSocket.
 *
 * Launch first:
 *   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222" bun run tauri dev
 * then:  bun scripts/tauri-cdp-e2e.ts
 *
 * Every assertion exercises the real frontend, the real Tauri plugin-store /
 * Stronghold persistence, and the real HTTP backend.
 */
const CDP_HTTP = process.env.CDP_HTTP ?? 'http://127.0.0.1:9222';
const SHOT = 'D:/Projects/GitHub/dungeon-master-ai/.cache-models/tauri_real_e2e.png';

function log(m: string): void {
  // eslint-disable-next-line no-console
  console.log(m);
}

interface Target {
  url: string;
  type: string;
  webSocketDebuggerUrl?: string;
}

async function appPageWsUrl(): Promise<string> {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const list = (await (await fetch(`${CDP_HTTP}/json/list`)).json()) as Target[];
      const page = list.find(
        (t) => t.type === 'page' && (t.url.includes('1420') || t.url.includes('tauri.localhost')),
      );
      if (page?.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
    } catch {
      /* app still booting */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error('app page (localhost:1420) not found over CDP');
}

class Cdp {
  private ws: WebSocket;
  private id = 0;
  private pending = new Map<number, (v: unknown) => void>();
  private ready: Promise<void>;

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.ready = new Promise((resolve, reject) => {
      this.ws.addEventListener('open', () => resolve());
      this.ws.addEventListener('error', (e) => reject(new Error(`ws error: ${String(e)}`)));
    });
    this.ws.addEventListener('message', (ev) => {
      const msg = JSON.parse(String((ev as MessageEvent).data)) as { id?: number; result?: unknown };
      if (msg.id !== undefined && this.pending.has(msg.id)) {
        this.pending.get(msg.id)?.(msg.result);
        this.pending.delete(msg.id);
      }
    });
  }

  async open(): Promise<void> {
    await this.ready;
  }

  send<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    const id = ++this.id;
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error(`CDP ${method} timed out`)), 60_000);
      this.pending.set(id, (v) => {
        clearTimeout(timer);
        resolve(v as T);
      });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  close(): void {
    this.ws.close();
  }
}

/** Evaluate a JS expression in the page and return its (by-value) result. */
async function evalInPage<T>(cdp: Cdp, expression: string): Promise<T> {
  const r = await cdp.send<{ result?: { value?: T } }>('Runtime.evaluate', {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  return r.result?.value as T;
}

/** Poll a boolean expression until true or timeout. */
async function waitFor(cdp: Cdp, expression: string, timeoutMs = 30_000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await evalInPage<boolean>(cdp, `Boolean(${expression})`)) return true;
    await new Promise((r) => setTimeout(r, 500));
  }
  return false;
}

async function main(): Promise<void> {
  const wsUrl = await appPageWsUrl();
  log(`connecting raw CDP to ${wsUrl}`);
  const cdp = new Cdp(wsUrl);
  await cdp.open();
  await cdp.send('Runtime.enable');
  await cdp.send('Page.enable');

  const checks: { name: string; ok: boolean; detail?: string }[] = [];
  const record = (name: string, ok: boolean, detail?: string) => {
    checks.push({ name, ok, detail });
    log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ` - ${detail}` : ''}`);
  };

  // 0. It really is the Tauri runtime (not the vite browser mock).
  const hasTauri = await evalInPage<boolean>(cdp, '!!window.__TAURI_INTERNALS__');
  record('real Tauri runtime present (window.__TAURI_INTERNALS__)', hasTauri);

  // 1. Real frontend rendered inside WebView2.
  const titlebar = await waitFor(cdp, `document.body.innerText.includes('DUNGEON MASTER AI')`);
  record('titlebar renders in the real WebView2', titlebar);

  // 2. Real backend reachable: SplashOverlay only drops after the real
  //    dmai-server answers /health, after which the chat composer mounts.
  //    Language-agnostic (the real app may be in RU): assert the splash is gone
  //    and the composer <textarea> is present.
  const composer = await waitFor(
    cdp,
    `!document.querySelector('.dm-splash') && !!document.querySelector('textarea')`,
  );
  record('composer visible / splash dismissed (real dmai-server /health passed)', composer);

  // 3. Real persistence: settings.json (onboarding_completed=true) read back
  //    through the real plugin-store / Stronghold hydration. Onboarding must
  //    not re-show (audit blocker 1, against the real vault).
  const noOnboarding = await evalInPage<boolean>(
    cdp,
    `!/Step \\d of \\d/i.test(document.body.innerText)`,
  );
  record('onboarding does not re-show (real Stronghold hydration gate)', noOnboarding);

  // Proof screenshot of the real app.
  const cap = await cdp.send<{ data: string }>('Page.captureScreenshot', { format: 'png' });
  await Bun.write(SHOT, Buffer.from(cap.data, 'base64'));
  log(`screenshot: ${SHOT}`);

  cdp.close();

  const failed = checks.filter((c) => !c.ok);
  log(`\n=== ${checks.length - failed.length}/${checks.length} real-Tauri checks passed ===`);
  if (failed.length > 0) process.exit(1);
}

main().catch((e) => {
  log(`ERROR: ${(e as Error).stack ?? e}`);
  process.exit(1);
});
