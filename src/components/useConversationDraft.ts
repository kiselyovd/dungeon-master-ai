import { type SetStateAction, useCallback, useEffect, useRef, useState } from 'react';
import type { StagedImage } from '../state/chat';
import { readConversationDraft, saveConversationDraft } from '../state/conversationDrafts';

interface Draft {
  text: string;
  attachments: StagedImage[];
}
const memory = new Map<string, Draft>();
const empty = (): Draft => ({ text: '', attachments: [] });

/** Scope changes cannot show or overwrite another participant's unsent input.
 * Attachments stay in memory; only bounded text is stored across a cold start. */
export function useConversationDraft(scope: string) {
  const [current, setCurrent] = useState({ scope, draft: memory.get(scope) ?? empty() });
  const [failure, setFailure] = useState(false);
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const version = useRef(0);
  const draft = current.scope === scope ? current.draft : (memory.get(scope) ?? empty());
  const save = useCallback(async (key: string, value: Draft) => {
    try {
      await saveConversationDraft(key, value.text);
      if (scopeRef.current === key) setFailure(false);
    } catch {
      if (scopeRef.current === key) setFailure(true);
    }
  }, []);
  useEffect(() => {
    let alive = true;
    const loadedVersion = version.current;
    setFailure(false);
    setCurrent({ scope, draft: memory.get(scope) ?? empty() });
    if (!memory.has(scope)) {
      void readConversationDraft(scope)
        .then((text) => {
          if (!alive || scopeRef.current !== scope || version.current !== loadedVersion) return;
          const value = { text, attachments: [] };
          memory.set(scope, value);
          setCurrent({ scope, draft: value });
        })
        .catch(() => {
          if (alive) setFailure(true);
        });
    }
    return () => {
      alive = false;
      if (timer.current) clearTimeout(timer.current);
      timer.current = null;
      const value = memory.get(scope);
      if (value) void save(scope, value);
    };
  }, [scope, save]);
  const update = (change: (previous: Draft) => Draft) => {
    const value = change(memory.get(scope) ?? draft);
    version.current++;
    memory.set(scope, value);
    setCurrent({ scope, draft: value });
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      timer.current = null;
      void save(scope, value);
    }, 300);
  };
  return {
    text: draft.text,
    attachments: draft.attachments,
    failure,
    setText: (value: SetStateAction<string>) =>
      update((previous) => ({
        ...previous,
        text: typeof value === 'function' ? value(previous.text) : value,
      })),
    setAttachments: (value: SetStateAction<StagedImage[]>) =>
      update((previous) => ({
        ...previous,
        attachments: typeof value === 'function' ? value(previous.attachments) : value,
      })),
    flush: () => save(scope, memory.get(scope) ?? draft),
  };
}
