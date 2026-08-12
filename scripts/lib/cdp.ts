import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';

export interface CdpTarget {
  url: string;
  type: string;
  title?: string;
  webSocketDebuggerUrl?: string;
}

interface CdpProtocolError {
  code: number;
  message: string;
  data?: string;
}

interface CdpMessage {
  id?: number;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: CdpProtocolError;
}

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
  method: string;
}

interface RuntimeEvaluateResult<T> {
  result?: {
    type?: string;
    value?: T;
    description?: string;
    unserializableValue?: string;
  };
  exceptionDetails?: {
    text?: string;
    exception?: { description?: string };
  };
}

type EventHandler = (params: unknown) => void;

const sleep = (milliseconds: number) =>
  new Promise<void>((resolvePromise) => setTimeout(resolvePromise, milliseconds));

export class CdpClient {
  private readonly socket: WebSocket;
  private readonly readyPromise: Promise<void>;
  private readonly pending = new Map<number, PendingRequest>();
  private readonly handlers = new Map<string, Set<EventHandler>>();
  private nextId = 0;
  private closed = false;

  constructor(url: string, connectTimeoutMs = 30_000) {
    this.socket = new WebSocket(url);
    this.readyPromise = new Promise<void>((resolvePromise, rejectPromise) => {
      const timeout = setTimeout(() => {
        rejectPromise(new Error(`CDP websocket connection timed out after ${connectTimeoutMs} ms`));
        this.socket.close();
      }, connectTimeoutMs);

      this.socket.addEventListener(
        'open',
        () => {
          clearTimeout(timeout);
          resolvePromise();
        },
        { once: true },
      );
      this.socket.addEventListener(
        'error',
        () => {
          clearTimeout(timeout);
          rejectPromise(new Error(`CDP websocket connection failed: ${url}`));
        },
        { once: true },
      );
    });

    this.socket.addEventListener('message', (event) => {
      let message: CdpMessage;
      try {
        message = JSON.parse(String((event as MessageEvent).data)) as CdpMessage;
      } catch (error) {
        this.rejectAll(new Error(`Invalid CDP response: ${String(error)}`));
        return;
      }

      if (message.id !== undefined) {
        const request = this.pending.get(message.id);
        if (!request) return;

        clearTimeout(request.timer);
        this.pending.delete(message.id);
        if (message.error) {
          const details = message.error.data ? ` (${message.error.data})` : '';
          request.reject(
            new Error(
              `CDP ${request.method} failed [${message.error.code}]: ${message.error.message}${details}`,
            ),
          );
        } else {
          request.resolve(message.result);
        }
        return;
      }

      if (message.method) {
        for (const handler of this.handlers.get(message.method) ?? []) {
          try {
            handler(message.params);
          } catch {
            // Event observers must never break the transport.
          }
        }
      }
    });

    this.socket.addEventListener('close', () => {
      this.closed = true;
      this.rejectAll(new Error('CDP websocket closed before the request completed'));
    });
    this.socket.addEventListener('error', () => {
      this.rejectAll(new Error('CDP websocket transport error'));
    });
  }

  async open(): Promise<void> {
    await this.readyPromise;
  }

  on(method: string, handler: EventHandler): () => void {
    const handlers = this.handlers.get(method) ?? new Set<EventHandler>();
    handlers.add(handler);
    this.handlers.set(method, handlers);
    return () => {
      handlers.delete(handler);
      if (handlers.size === 0) this.handlers.delete(method);
    };
  }

  send<T = unknown>(
    method: string,
    params: Record<string, unknown> = {},
    timeoutMs = 60_000,
  ): Promise<T> {
    if (this.closed || this.socket.readyState !== WebSocket.OPEN) {
      return Promise.reject(new Error(`Cannot send CDP ${method}: websocket is not open`));
    }

    const id = ++this.nextId;
    return new Promise<T>((resolvePromise, rejectPromise) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        rejectPromise(new Error(`CDP ${method} timed out after ${timeoutMs} ms`));
      }, timeoutMs);

      this.pending.set(id, {
        resolve: (value) => resolvePromise(value as T),
        reject: rejectPromise,
        timer,
        method,
      });

      try {
        this.socket.send(JSON.stringify({ id, method, params }));
      } catch (error) {
        clearTimeout(timer);
        this.pending.delete(id);
        rejectPromise(new Error(`Failed to send CDP ${method}: ${String(error)}`));
      }
    });
  }

  close(): void {
    if (!this.closed) this.socket.close();
  }

  private rejectAll(error: Error): void {
    for (const request of this.pending.values()) {
      clearTimeout(request.timer);
      request.reject(error);
    }
    this.pending.clear();
  }
}

export async function findAppPageWebSocket(options: {
  cdpHttp?: string;
  timeoutMs?: number;
  urlHints?: string[];
} = {}): Promise<string> {
  const cdpHttp = options.cdpHttp ?? process.env.CDP_HTTP ?? 'http://127.0.0.1:9222';
  const timeoutMs = options.timeoutMs ?? 60_000;
  const configuredHints = process.env.TAURI_APP_URL_HINTS
    ?.split(',')
    .map((value) => value.trim())
    .filter(Boolean);
  const urlHints = options.urlHints ?? configuredHints ?? ['tauri.localhost', 'localhost:1420', '127.0.0.1:1420'];
  const deadline = Date.now() + timeoutMs;
  let lastError = 'CDP target list was empty';

  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${cdpHttp}/json/list`);
      if (!response.ok) {
        lastError = `${response.status} ${response.statusText}`;
      } else {
        const targets = (await response.json()) as CdpTarget[];
        const page = targets.find(
          (target) =>
            target.type === 'page' &&
            target.webSocketDebuggerUrl &&
            urlHints.some((hint) => target.url.includes(hint)),
        );
        if (page?.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
        lastError = `No page matched hints: ${urlHints.join(', ')}`;
      }
    } catch (error) {
      lastError = String(error);
    }
    await sleep(500);
  }

  throw new Error(`Tauri app page was not exposed by ${cdpHttp}: ${lastError}`);
}

export async function evaluateInPage<T>(
  cdp: CdpClient,
  expression: string,
  timeoutMs = 60_000,
): Promise<T> {
  const result = await cdp.send<RuntimeEvaluateResult<T>>(
    'Runtime.evaluate',
    {
      expression,
      returnByValue: true,
      awaitPromise: true,
      userGesture: true,
    },
    timeoutMs,
  );

  if (result.exceptionDetails) {
    const description =
      result.exceptionDetails.exception?.description ?? result.exceptionDetails.text ?? 'unknown exception';
    throw new Error(`Page evaluation failed: ${description}`);
  }
  return result.result?.value as T;
}

export async function waitForPageCondition(
  cdp: CdpClient,
  expression: string,
  timeoutMs = 30_000,
  intervalMs = 500,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      if (await evaluateInPage<boolean>(cdp, `Boolean(${expression})`)) return true;
    } catch (error) {
      lastError = error;
    }
    await sleep(intervalMs);
  }
  if (lastError) throw lastError;
  return false;
}

export function defaultArtifactPath(fileName: string): string {
  return resolve(process.env.TAURI_E2E_ARTIFACT_DIR ?? join(process.cwd(), '.artifacts'), fileName);
}

export async function captureScreenshot(cdp: CdpClient, outputPath: string): Promise<void> {
  const capture = await cdp.send<{ data: string }>('Page.captureScreenshot', {
    format: 'png',
    fromSurface: true,
    captureBeyondViewport: false,
  });
  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, Buffer.from(capture.data, 'base64'));
}
