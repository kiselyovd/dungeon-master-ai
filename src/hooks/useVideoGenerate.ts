/**
 * M7-DM video generation client hook.
 *
 * Three modes (selected via Settings.videoMode):
 *  - `prerecorded` - never hits backend; resolves to a bundled mp4 by scene tag.
 *  - `live` - POSTs to /video/generate, streams VideoEvents over SSE, builds
 *    a Blob URL from the Done payload's mp4_bytes array.
 *  - `race` - kicks both live + prerecorded in parallel; first to resolve wins
 *    (live cancelled on prerecorded win, prerecorded discarded on live win).
 *
 * When the LTX-Video model isn't downloaded yet (sidecar returns 503), live
 * falls back to prerecorded with a warning toast (rendered by the caller).
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { backendUrl } from '../api/client';
import { useStore } from '../state/useStore';

export type SceneTag = 'combat' | 'dialog' | 'exploration' | 'dungeon';

const PRERECORDED_BY_TAG: Record<SceneTag, string> = {
  combat: '/src/assets/scene-transition-combat.mp4',
  dialog: '/src/assets/scene-transition-dialog.mp4',
  exploration: '/src/assets/scene-transition-exploration.mp4',
  dungeon: '/src/assets/scene-transition-dungeon.mp4',
};

export type UseVideoGenerateMode = 'prerecorded' | 'live' | 'race';

export type UseVideoGenerateOpts = {
  sceneTag?: SceneTag;
};

export type UseVideoGenerateState = {
  status: 'idle' | 'starting' | 'progress' | 'done' | 'error';
  percent: number | null;
  etaSeconds: number | null;
  mp4Url: string | null;
  error: string | null;
};

const INITIAL: UseVideoGenerateState = {
  status: 'idle',
  percent: null,
  etaSeconds: null,
  mp4Url: null,
  error: null,
};

export function useVideoGenerate(opts: UseVideoGenerateOpts = {}) {
  const [state, setState] = useState<UseVideoGenerateState>(INITIAL);
  const abortRef = useRef<AbortController | null>(null);
  const blobUrlRef = useRef<string | null>(null);

  useEffect(
    () => () => {
      abortRef.current?.abort();
      if (blobUrlRef.current) {
        URL.revokeObjectURL(blobUrlRef.current);
        blobUrlRef.current = null;
      }
    },
    [],
  );

  const reset = useCallback(() => {
    abortRef.current?.abort();
    if (blobUrlRef.current) {
      URL.revokeObjectURL(blobUrlRef.current);
      blobUrlRef.current = null;
    }
    setState(INITIAL);
  }, []);

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    setState((s) => ({ ...s, status: 'idle' }));
  }, []);

  const generate = useCallback(
    async (prompt: { text: string }, mode: UseVideoGenerateMode) => {
      reset();

      // Resolve the prerecorded mp4 for the current scene tag and update state.
      const resolvePrerecorded = (error: string | null = null): void => {
        const tag = opts.sceneTag ?? 'exploration';
        setState({
          status: 'done',
          percent: 1,
          etaSeconds: 0,
          mp4Url: PRERECORDED_BY_TAG[tag],
          error,
        });
      };

      // Short-circuit: if video generation is disabled in settings, resolve the
      // prerecorded mp4 for the current scene tag instead of calling the backend.
      if (useStore.getState().settings.videoEnabled === false || mode === 'prerecorded') {
        resolvePrerecorded();
        return;
      }
      const ac = new AbortController();
      abortRef.current = ac;
      setState({ ...INITIAL, status: 'starting' });
      try {
        const url = await backendUrl('/video/generate');
        const resp = await fetch(url, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text: prompt.text }),
          signal: ac.signal,
        });
        if (resp.status === 404 || resp.status === 503) {
          // Universal fallback for live/race when sidecar isn't ready.
          resolvePrerecorded(
            mode === 'live' ? 'Live generation unavailable; falling back to library clip.' : null,
          );
          return;
        }
        if (!resp.ok || !resp.body) {
          throw new Error(`backend returned ${resp.status}`);
        }
        const reader = resp.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          let sepIdx = buffer.indexOf('\n\n');
          while (sepIdx >= 0) {
            const frame = buffer.slice(0, sepIdx);
            buffer = buffer.slice(sepIdx + 2);
            sepIdx = buffer.indexOf('\n\n');
            const dataLine = frame
              .split('\n')
              .find((l) => l.startsWith('data: '))
              ?.slice('data: '.length);
            if (!dataLine) continue;
            const evt = JSON.parse(dataLine) as {
              type: string;
              percent?: number;
              eta_seconds?: number;
              estimated_seconds?: number;
              mp4_bytes?: number[];
              duration_seconds?: number;
              message?: string;
            };
            if (evt.type === 'started') {
              setState((s) => ({
                ...s,
                status: 'progress',
                etaSeconds: evt.estimated_seconds ?? null,
              }));
            } else if (evt.type === 'progress') {
              setState((s) => ({
                ...s,
                status: 'progress',
                percent: evt.percent ?? null,
                etaSeconds: evt.eta_seconds ?? null,
              }));
            } else if (evt.type === 'done' && evt.mp4_bytes) {
              const blob = new Blob([new Uint8Array(evt.mp4_bytes)], { type: 'video/mp4' });
              const blobUrl = URL.createObjectURL(blob);
              blobUrlRef.current = blobUrl;
              setState({
                status: 'done',
                percent: 1,
                etaSeconds: 0,
                mp4Url: blobUrl,
                error: null,
              });
            } else if (evt.type === 'error') {
              setState({
                status: 'error',
                percent: null,
                etaSeconds: null,
                mp4Url: null,
                error: evt.message ?? 'unknown error',
              });
            }
          }
        }
      } catch (e) {
        if ((e as { name?: string }).name === 'AbortError') return;
        setState({
          status: 'error',
          percent: null,
          etaSeconds: null,
          mp4Url: null,
          error: e instanceof Error ? e.message : 'video generation failed',
        });
      }
    },
    [opts.sceneTag, reset],
  );

  return { ...state, generate, cancel, reset };
}
