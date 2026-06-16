/**
 * Tiny interactive CDP driver for the running WebView2 app (port 9222).
 * Connects fresh per invocation, does ONE thing, exits.
 *
 *   bun scripts/cdp.ts list                 # list CDP targets
 *   bun scripts/cdp.ts eval '<expr>'        # Runtime.evaluate, print JSON result
 *   bun scripts/cdp.ts evalfile <path>      # run JS from a file (big expressions)
 *   bun scripts/cdp.ts shot <png-path>      # Page.captureScreenshot -> file
 *
 * Expressions run with returnByValue + awaitPromise, so `(async()=>{...})()`
 * works and the resolved value is printed.
 */
import { readFileSync } from 'node:fs';

const CDP_HTTP = 'http://127.0.0.1:9222';
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

interface T {
  url: string;
  type: string;
  title: string;
  webSocketDebuggerUrl?: string;
}

async function targets(): Promise<T[]> {
  return (await (await fetch(`${CDP_HTTP}/json/list`)).json()) as T[];
}

async function pageWs(): Promise<string> {
  const d = Date.now() + 15_000;
  while (Date.now() < d) {
    try {
      const list = await targets();
      const p =
        list.find((t) => t.type === 'page' && t.url.includes('1420')) ??
        list.find((t) => t.type === 'page' && /^https?:|^tauri:/.test(t.url));
      if (p?.webSocketDebuggerUrl) return p.webSocketDebuggerUrl;
    } catch {}
    await sleep(400);
  }
  throw new Error('no app page target');
}

class Cdp {
  ws: WebSocket;
  id = 0;
  pend = new Map<number, (v: unknown) => void>();
  ready: Promise<void>;
  constructor(u: string) {
    this.ws = new WebSocket(u);
    this.ready = new Promise((res) => this.ws.addEventListener('open', () => res()));
    this.ws.addEventListener('message', (e) => {
      const m = JSON.parse(String((e as MessageEvent).data)) as { id?: number; result?: unknown };
      if (m.id !== undefined) this.pend.get(m.id)?.(m.result);
    });
  }
  send<R>(method: string, params: Record<string, unknown> = {}): Promise<R> {
    const id = ++this.id;
    return new Promise<R>((res) => {
      this.pend.set(id, (v) => res(v as R));
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
}

async function main(): Promise<void> {
  const [cmd, arg] = process.argv.slice(2);

  if (cmd === 'list') {
    const list = await targets();
    for (const t of list) console.log(`${t.type}\t${t.url}\t${(t.title || '').slice(0, 40)}`);
    return;
  }

  const cdp = new Cdp(await pageWs());
  await cdp.ready;
  await cdp.send('Runtime.enable');
  await cdp.send('Page.enable');

  if (cmd === 'shot') {
    const c = await cdp.send<{ data: string }>('Page.captureScreenshot', { format: 'png' });
    await Bun.write(arg, Buffer.from(c.data, 'base64'));
    console.log(`shot -> ${arg}`);
    return;
  }

  if (cmd === 'eval' || cmd === 'evalfile') {
    const expr = cmd === 'evalfile' ? readFileSync(arg, 'utf8') : arg;
    const r = await cdp.send<{ result?: { value?: unknown }; exceptionDetails?: unknown }>(
      'Runtime.evaluate',
      { expression: expr, returnByValue: true, awaitPromise: true },
    );
    if (r.exceptionDetails) {
      console.log(`EXCEPTION: ${JSON.stringify(r.exceptionDetails)}`);
      process.exit(2);
    }
    console.log(typeof r.result?.value === 'string' ? r.result.value : JSON.stringify(r.result?.value));
    return;
  }

  throw new Error(`unknown cmd: ${cmd}`);
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.log(`ERROR ${(e as Error).stack ?? e}`);
    process.exit(1);
  });
