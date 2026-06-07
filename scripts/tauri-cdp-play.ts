/**
 * Real-Tauri full play flow over raw CDP: switch the chat provider to the local
 * runtime, START it from the UI, save, then send a player action and wait for
 * the live Dungeon Master (local Gemma) to respond. Drives the actual WebView2
 * window with the real dmai-server behind it.
 *
 * Env: BPORT = dmai-server backend port (for runtime-readiness polling).
 * Prereq: app running with --remote-debugging-port=9222.
 */
import { readFileSync } from 'node:fs';

const CDP_HTTP = 'http://127.0.0.1:9222';
const SHOT = 'D:/Projects/GitHub/dungeon-master-ai/.cache-models/tauri_play.png';
const PROMPT =
  process.env.PLAY_PROMPT ??
  'I push open the tavern door and look around the common room. Describe what I see.';
const BPORT =
  process.env.BPORT ?? readFileSync('/tmp/bport.txt', 'utf8').trim();

const log = (m: string): void => console.log(m); // eslint-disable-line no-console
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

interface T { url: string; type: string; webSocketDebuggerUrl?: string }
async function pageWs(): Promise<string> {
  const d = Date.now() + 30_000;
  while (Date.now() < d) {
    try {
      const list = (await (await fetch(`${CDP_HTTP}/json/list`)).json()) as T[];
      const p = list.find((t) => t.type === 'page' && t.url.includes('1420'));
      if (p?.webSocketDebuggerUrl) return p.webSocketDebuggerUrl;
    } catch {}
    await sleep(500);
  }
  throw new Error('no app page');
}
class Cdp {
  ws: WebSocket; id = 0; pend = new Map<number, (v: unknown) => void>(); ready: Promise<void>;
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
    return new Promise<R>((res) => { this.pend.set(id, (v) => res(v as R)); this.ws.send(JSON.stringify({ id, method, params })); });
  }
}
let cdp: Cdp;
async function ev<R>(expression: string): Promise<R> {
  const r = await cdp.send<{ result?: { value?: R } }>('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  return r.result?.value as R;
}
async function waitUi(expr: string, ms: number): Promise<boolean> {
  const d = Date.now() + ms;
  while (Date.now() < d) { if (await ev<boolean>(`Boolean(${expr})`)) return true; await sleep(600); }
  return false;
}
async function clickInDialog(re: string): Promise<boolean> {
  return ev<boolean>(`(() => { const b=[...document.querySelectorAll('[role=dialog] button')].find(b=>${re}.test((b.textContent||'').trim())); if(b)b.click(); return !!b; })()`);
}

async function backendReady(): Promise<boolean> {
  // Wait for BOTH the LLM and the image sidecar: /settings/v2 builds the image
  // provider too and 400s if the media sidecar URL is not yet set (i.e. the
  // image runtime is still starting). The image sidecar (Python+torch) starts
  // slower than the LLM.
  try {
    const s = (await (await fetch(`http://127.0.0.1:${BPORT}/local/runtime/status`)).json()) as {
      llm?: { state?: string };
      image?: { state?: string };
    };
    return s.llm?.state === 'ready';
  } catch {
    return false;
  }
}

async function main(): Promise<void> {
  cdp = new Cdp(await pageWs());
  await cdp.ready;
  await cdp.send('Runtime.enable');
  await cdp.send('Page.enable');

  if (!(await waitUi(`document.body.textContent.includes('DUNGEON MASTER AI')`, 180_000)))
    throw new Error('app did not render');
  log('app rendered.');

  // Install capture for the settings + agent endpoints.
  await ev(`(() => { if(window.__c)return true; window.__c=true; window.__sv=null; window.__at=null; const of=window.fetch; window.fetch=async(...a)=>{const u=String(a[0]); const r=await of(...a); try{ if(u.includes('/settings/v2')) window.__sv={status:r.status}; if(u.includes('/agent/turn')) window.__at={status:r.status}; }catch{} return r;}; return true; })()`);

  // 1. Open Settings and switch provider to local-mistralrs.
  await ev(`(() => { const b=[...document.querySelectorAll('button')].find(b=>/Настройк|Settings/i.test(b.getAttribute('aria-label')||'')); if(b)b.click(); return !!b; })()`);
  await waitUi(`document.querySelector('[role="dialog"] select')`, 10_000);
  await ev(`(() => { const s=[...document.querySelectorAll('[role=dialog] select')].find(s=>[...s.options].some(o=>o.value==='local-mistralrs')); if(!s)return false; const set=Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype,'value').set; set.call(s,'local-mistralrs'); s.dispatchEvent(new Event('change',{bubbles:true})); return true; })()`);
  log('provider switched to local-mistralrs in the form.');
  await sleep(1500);

  // 2. Start the runtime from the Local LLM tab (idempotent if already running).
  const started = await clickInDialog('/^(Start runtimes|Start|Запустить рантаймы|Запустить)$/i');
  log(`clicked Start runtimes: ${started}`);

  // 3. Wait until the backend reports the LLM runtime ready (Gemma load).
  let ready = false;
  for (let i = 0; i < 30; i++) { if (await backendReady()) { ready = true; log(`runtime ready (~${i * 5}s)`); break; } await sleep(5000); }
  if (!ready) log('WARN runtime not ready after wait; saving anyway');
  // Let the frontend's 5s poll pick up the ready state (localMode.runtime) so
  // buildChatProvidersSlice does not throw "local runtime is not ready".
  await sleep(9000);

  // 4. Save -> applies the local provider (with the runtime port) to the backend.
  await clickInDialog('/^(Сохранить|Save)$/');
  const closed = await waitUi(`!document.querySelector('[role="dialog"]')`, 15_000);
  const sv = await ev<{ status: number } | null>(`window.__sv`);
  log(`settings dialog closed: ${closed}; /settings/v2 -> ${sv ? sv.status : 'NOT POSTED'}`);
  const banner = await ev<string>(`(document.querySelector('[data-testid="settings-save-error"]')?.textContent || '').slice(0,200)`);
  if (banner) log(`save error banner: ${banner}`);

  // 5. Send the player's action.
  await sleep(1000);
  await ev(`(() => { const ta=document.querySelector('textarea'); const set=Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype,'value').set; set.call(ta,${JSON.stringify(PROMPT)}); ta.dispatchEvent(new Event('input',{bubbles:true})); ta.focus(); ta.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true})); return true; })()`);
  log(`sent: "${PROMPT}". Waiting for the Dungeon Master...`);

  // 6. Wait for an assistant bubble OR a reasoning pill (Gemma is a thinking model).
  const reply = await (async () => {
    const d = Date.now() + 240_000;
    while (Date.now() < d) {
      const t = await ev<string>(`(() => { const b=document.querySelector('[data-testid="bubble"][data-role="assistant"]'); const bt=b?(b.textContent||'').trim():''; const rp=document.querySelector('[data-testid="reasoning-body"],[data-testid="reasoning-thinking"]'); const rt=rp?(rp.textContent||'').trim():''; return bt.length>8?('TEXT:'+bt):(rt.length>8?('REASONING:'+rt):''); })()`);
      if (t) return t;
      await sleep(1500);
    }
    return '';
  })();

  await cdp.send<{ data: string }>('Page.captureScreenshot', { format: 'png' }).then(async (c) => { await Bun.write(SHOT, Buffer.from(c.data, 'base64')); });
  log(`screenshot: ${SHOT}`);

  const at = await ev<{ status: number } | null>(`window.__at`);
  log(`/agent/turn -> ${at ? at.status : 'NOT POSTED'}`);
  if (reply) { log(`\n=== DUNGEON MASTER (live Gemma) ===\n${reply.slice(0, 700)}\n`); log('PASS live DM response'); }
  else { const chat = await ev<string>(`(document.querySelector('[data-testid="chat-history"]')?.textContent||'').slice(-400)`); log(`\nFAIL no DM response. chat tail:\n${chat}`); process.exit(1); }
}
main().catch((e) => { log(`ERROR ${(e as Error).stack ?? e}`); process.exit(1); });
