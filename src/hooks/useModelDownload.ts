import { useCallback, useEffect, useRef } from 'react';
import type { ModelId } from '../state/localMode';
import { useStore } from '../state/useStore';

const apiBase = (): string => '';

export function useModelDownload(modelId: ModelId) {
  const setDownloadState = useStore((s) => s.localMode.setDownloadState);
  const esRef = useRef<EventSource | null>(null);

  useEffect(() => {
    return () => {
      esRef.current?.close();
      esRef.current = null;
    };
  }, []);

  const start = useCallback(async () => {
    const resp = await fetch(`${apiBase()}/local/download/${modelId}`, { method: 'POST' });
    if (!resp.ok) {
      const reason = `download start failed: ${resp.status}`;
      setDownloadState(modelId, { state: 'failed', reason });
      throw new Error(reason);
    }
    setDownloadState(modelId, { state: 'downloading', bytesDone: 0, totalBytes: null });
    esRef.current?.close();
    const es = new EventSource(`${apiBase()}/local/download/${modelId}/progress`);
    esRef.current = es;
    es.onmessage = (ev) => {
      try {
        const data = JSON.parse(ev.data);
        if (data.kind === 'progress') {
          setDownloadState(modelId, {
            state: 'downloading',
            bytesDone: data.bytes_done,
            totalBytes: data.total_bytes ?? null,
          });
        } else if (data.kind === 'completed') {
          setDownloadState(modelId, { state: 'completed', bytesTotal: data.bytes_total });
          es.close();
        } else if (data.kind === 'failed') {
          setDownloadState(modelId, { state: 'failed', reason: data.reason ?? 'unknown' });
          es.close();
        }
      } catch (err) {
        console.warn('progress parse error', err);
      }
    };
    es.onerror = () => {
      es.close();
    };
  }, [modelId, setDownloadState]);

  const cancel = useCallback(async () => {
    esRef.current?.close();
    esRef.current = null;
    await fetch(`${apiBase()}/local/download/${modelId}`, { method: 'DELETE' });
    setDownloadState(modelId, { state: 'idle' });
  }, [modelId, setDownloadState]);

  return { start, cancel };
}
