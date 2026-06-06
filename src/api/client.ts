import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

let cachedPort: number | null = null;
let waitingResolvers: Array<(port: number) => void> = [];

/** Upper bound on how long a caller waits for the backend-ready event before
 * its promise rejects. Without this, `backendUrl` (and every caller's loading
 * state) hangs forever if the sidecar never announces a port. */
const BACKEND_PORT_TIMEOUT_MS = 10_000;

async function getBackendPort(): Promise<number> {
  if (cachedPort !== null) return cachedPort;

  // Try synchronous fetch first; the sidecar may already be ready.
  const port = (await invoke<number | null>('backend_port').catch(() => null)) ?? null;
  if (port !== null) {
    cachedPort = port;
    return port;
  }

  // Otherwise wait for the backend-ready event from Rust, but do not hang
  // forever - reject after a bounded wait so callers' loading state resolves.
  return new Promise<number>((resolve, reject) => {
    let timer: ReturnType<typeof setTimeout>;
    const wrapped = (p: number): void => {
      clearTimeout(timer);
      resolve(p);
    };
    timer = setTimeout(() => {
      waitingResolvers = waitingResolvers.filter((r) => r !== wrapped);
      reject(new Error('timed out waiting for backend port'));
    }, BACKEND_PORT_TIMEOUT_MS);
    waitingResolvers.push(wrapped);
  });
}

export async function initBackendListener(): Promise<UnlistenFn> {
  return listen<number>('backend-ready', (e) => {
    cachedPort = e.payload;
    waitingResolvers.forEach((r) => {
      r(e.payload);
    });
    waitingResolvers = [];
  });
}

export async function backendUrl(path: string): Promise<string> {
  const port = await getBackendPort();
  return `http://127.0.0.1:${port}${path}`;
}

export function setBackendPortForTesting(port: number | null): void {
  cachedPort = port;
}

/**
 * Synchronous access to the cached backend port for UI surfaces (cURL copy,
 * status bar) that need a value right away. Returns `null` if the sidecar
 * has not announced its port yet.
 */
export function getCachedBackendPort(): number | null {
  return cachedPort;
}
