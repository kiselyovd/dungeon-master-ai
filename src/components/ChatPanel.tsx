import {
  type KeyboardEvent,
  type ClipboardEvent as ReactClipboardEvent,
  type DragEvent as ReactDragEvent,
  useCallback,
  useEffect,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import type { ChatErrorCode } from '../api/errors';
import { useAgentTurn } from '../hooks/useAgentTurn';
import { useSession } from '../hooks/useSession';
import { useStickyScroll } from '../hooks/useStickyScroll';
import { fileToDataUrl } from '../lib/fileToDataUrl';
import type { ChatMessage, ChatStreamEvent, MessagePart, StagedImage } from '../state/chat';
import type { ToolLogEntry } from '../state/toolLog';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';
import styles from './ChatPanel.module.css';
import { ComposerAttachments } from './ComposerAttachments';
import { MessageBubble } from './MessageBubble';
import { ReasoningPill } from './ReasoningPill';
import { ToolCallCard } from './ToolCallCard';
import { TypingIndicator } from './TypingIndicator';

const VALID_IMAGE_MIMES = new Set(['image/png', 'image/jpeg', 'image/webp']);

const MAX_IMAGE_BYTES = 5 * 1024 * 1024;
const MAX_IMAGES_PER_MESSAGE = 4;

/** Map a ChatStreamEvent to the ToolLogEntry shape expected by ToolCallCard. */
function streamEventToLogEntry(event: ChatStreamEvent): ToolLogEntry {
  return {
    id: event.id,
    toolName: event.toolName,
    args: event.args,
    result: event.result,
    isError: event.isError,
    round: event.round,
    // Intentional empty shim: ChatStreamEvent carries no timestamp and ToolCallCard does not display it.
    timestamp: '',
    handledBy: 'engine',
  };
}

type MergedItem = { kind: 'message'; item: ChatMessage } | { kind: 'event'; item: ChatStreamEvent };

export function ChatPanel() {
  const { t } = useTranslation('chat');
  const { t: tErrors } = useTranslation('errors');
  const { t: tTools } = useTranslation('tools');
  const { send, cancel } = useAgentTurn();
  const messages = useStore((s) => s.chat.messages);
  const truncateTo = useStore((s) => s.chat.truncateTo);
  const chatStreamEvents = useStore((s) => s.chat.chatStreamEvents);
  const streamingAssistant = useStore((s) => s.chat.streamingAssistant);
  const isStreaming = useStore((s) => s.chat.isStreaming);
  const lastError = useStore((s) => s.chat.lastError);
  const { refetch: refetchSession } = useSession();
  const sessionLoadError = useStore((s) => s.session.loadError);
  // M7-DM: vision input gate. When the user has explicitly disabled multimodal
  // input in Settings.Chat (or the active model lacks vision capability), the
  // paperclip is hidden and paste/drag of image files is rejected with a hint
  // in the staging error slot. Text-only chat is unaffected.
  const visionEnabled = useStore((s) => s.settings.visionEnabled);
  const streamingReasoning = useStore((s) => s.chat.streamingReasoning);
  const [draft, setDraft] = useState('');
  const [staged, setStaged] = useState<StagedImage[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [stagingError, setStagingError] = useState<string | null>(null);
  const [isRetrying, setIsRetrying] = useState(false);
  const { ref: historyRef, onScroll, scrollToBottom, reset: resetScroll } = useStickyScroll(100);

  // biome-ignore lint/correctness/useExhaustiveDependencies: scrollToBottom intentionally re-fires only when conversation length changes
  useEffect(() => {
    scrollToBottom();
  }, [messages.length, chatStreamEvents.length, streamingAssistant, scrollToBottom]);

  // Auto-dismiss the image staging error after 4 seconds.
  useEffect(() => {
    if (stagingError === null) return;
    const id = setTimeout(() => setStagingError(null), 4000);
    return () => clearTimeout(id);
  }, [stagingError]);

  const addFiles = useCallback(
    async (files: FileList | File[]) => {
      const list = Array.from(files);
      let didError: string | null = null;
      for (const file of list) {
        if (staged.length + 1 > MAX_IMAGES_PER_MESSAGE) {
          didError = t('too_many_images');
          break;
        }
        if (!VALID_IMAGE_MIMES.has(file.type)) {
          didError = t('image_unsupported');
          continue;
        }
        if (file.size > MAX_IMAGE_BYTES) {
          didError = t('image_too_large');
          continue;
        }
        try {
          const dataUrl = await fileToDataUrl(file);
          setStaged((prev) =>
            prev.length >= MAX_IMAGES_PER_MESSAGE
              ? prev
              : [
                  ...prev,
                  {
                    mime: file.type,
                    dataUrl,
                    name: file.name,
                    sizeBytes: file.size,
                  },
                ],
          );
        } catch {
          didError = t('image_unsupported');
        }
      }
      if (didError !== null) setStagingError(didError);
    },
    [staged.length, t],
  );

  const removeStaged = useCallback((index: number) => {
    setStaged((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const canSend = !isStreaming && draft.trim().length > 0;

  // Build a retry handler for a finalized assistant bubble identified by its
  // message id. Clicking Retry must:
  //   1. Find the user message immediately preceding this assistant turn.
  //   2. truncateTo(userMessageId) - removes the user message AND the full
  //      assistant turn (and any trailing messages) so there is no orphan.
  //   3. send(userMessage.content) - re-appends the user message and replays.
  // The user message is removed then re-added in the same tick, so visually it
  // "stays" with no duplicate (see Nuance 1 in the B5 task brief).
  const makeRetryHandler = useCallback(
    (assistantMessageId: string) => () => {
      // Read the live value to avoid a stale-closure race: two rapid clicks
      // before re-render would both pass a closed-over isStreaming=false.
      if (useStore.getState().chat.isStreaming) return;
      const currentMessages = useStore.getState().chat.messages;
      const aIdx = currentMessages.findIndex((m) => m.id === assistantMessageId);
      if (aIdx === -1) return;
      // Walk backward from the assistant message to find the triggering user message.
      let userMsg: (typeof currentMessages)[number] | undefined;
      for (let i = aIdx - 1; i >= 0; i--) {
        if (currentMessages[i]?.role === 'user') {
          userMsg = currentMessages[i];
          break;
        }
      }
      if (!userMsg) return;
      const text = userMsg.content;
      const userId = userMsg.id;
      // Show the "Retrying..." indicator before truncating so there is no
      // window where history is cleared but the indicator is not yet visible.
      setIsRetrying(true);
      // Remove the user message and everything after it (including the full
      // assistant turn), then replay via send() which re-appends the user message.
      truncateTo(userId);
      void (async () => {
        try {
          await send(text);
        } finally {
          setIsRetrying(false);
        }
      })();
    },
    [truncateTo, send],
  );

  const onSend = async () => {
    if (!canSend) return;
    const text = draft;
    setDraft('');
    // Convert staged images to wire MessageParts (strip the data-URL prefix to
    // the bare base64 the backend expects). They ride the dedicated `images`
    // field of the agent turn (F2 - vision wired end to end).
    const images: MessagePart[] = staged.map((img) => ({
      type: 'image',
      mime: img.mime,
      data_b64: img.dataUrl.replace(/^data:[^;]+;base64,/, ''),
      name: img.name ?? null,
    }));
    setStaged([]);
    setStagingError(null);
    await send(text, images.length > 0 ? images : undefined);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void onSend();
    } else if (e.key === 'Escape' && isStreaming) {
      e.preventDefault();
      cancel();
      resetScroll();
    }
  };

  const onPaste = (e: ReactClipboardEvent<HTMLTextAreaElement>) => {
    const files: File[] = [];
    for (const item of Array.from(e.clipboardData.items)) {
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file) files.push(file);
      }
    }
    if (files.length > 0) {
      e.preventDefault();
      if (!visionEnabled) {
        setStagingError(t('vision_disabled'));
        return;
      }
      void addFiles(files);
    }
  };

  const onDragOver = (e: ReactDragEvent<HTMLDivElement>) => {
    if (e.dataTransfer.types.includes('Files')) {
      e.preventDefault();
      setIsDragging(true);
    }
  };
  const onDragLeave = () => setIsDragging(false);
  const onDrop = (e: ReactDragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(false);
    if (e.dataTransfer.files.length === 0) return;
    if (!visionEnabled) {
      setStagingError(t('vision_disabled'));
      return;
    }
    void addFiles(e.dataTransfer.files);
  };

  // Window-level ESC also aborts an in-flight stream so the user can hit it
  // even when focus has wandered out of the textarea.
  useEffect(() => {
    if (!isStreaming) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') {
        cancel();
        resetScroll();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isStreaming, cancel, resetScroll]);

  const isEmptyChat = messages.length === 0 && streamingAssistant === null && !isStreaming;

  // Merge finalized messages and inline tool-call events into one ordered list.
  // Both draw sequenceIndex from the shared _nextSeq counter in the slice.
  const mergedStream: MergedItem[] = [
    ...messages.map((m) => ({ kind: 'message' as const, item: m })),
    ...chatStreamEvents.map((e) => ({ kind: 'event' as const, item: e })),
  ].sort((a, b) => (a.item.sequenceIndex ?? 0) - (b.item.sequenceIndex ?? 0));

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: drag-drop surface; keyboard alternative is paste/Ctrl+V into the textarea
    <div
      className={`${styles.panel} ${isDragging ? styles.dragging : ''}`}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      <div className={styles.header}>
        <span className={styles.headerTitle}>
          <Icons.Sparkle size={14} />
          <span>{t('header_title')}</span>
        </span>
      </div>

      <div
        ref={historyRef}
        className={styles.history}
        onScroll={onScroll}
        data-testid="chat-history"
      >
        {sessionLoadError !== null && (
          <div role="alert" className={`${styles.sessionLoadError} dm-chat-error`}>
            <span className={styles.sessionLoadErrorText}>{t('session_load_error')}</span>
            <button type="button" className={styles.sessionLoadErrorRetry} onClick={refetchSession}>
              {t('retry')}
            </button>
          </div>
        )}
        {isEmptyChat && (
          <div className={styles.welcome} aria-hidden="true">
            <div className={styles.welcomeOrnament} />
            <div className={styles.welcomeTitle}>{t('welcome_title')}</div>
            <div className={styles.welcomeText}>{t('welcome_text')}</div>
            <div className={styles.welcomeOrnament} />
          </div>
        )}
        {mergedStream.map((entry) => {
          if (entry.kind === 'event') {
            const toolLabel = tTools(entry.item.toolName, { defaultValue: entry.item.toolName });
            return (
              <ToolCallCard
                key={entry.item.id}
                entry={streamEventToLogEntry(entry.item)}
                label={toolLabel}
              />
            );
          }
          const m = entry.item;
          // Only pass onRetry/retryDisabled to finalized assistant bubbles;
          // user/system bubbles never show the retry tray (MessageBubble guards
          // on isNarrator). exactOptionalPropertyTypes requires we omit the prop
          // entirely rather than passing undefined.
          const assistantProps =
            m.role === 'assistant'
              ? { onRetry: makeRetryHandler(m.id), retryDisabled: isStreaming }
              : {};
          return m.parts !== undefined ? (
            <MessageBubble key={m.id} chatRole={m.role} parts={m.parts} {...assistantProps}>
              {m.content}
            </MessageBubble>
          ) : (
            <MessageBubble key={m.id} chatRole={m.role} {...assistantProps}>
              {m.content}
            </MessageBubble>
          );
        })}
        {isRetrying && (
          <div aria-live="polite" className={styles.retryingLabel} data-testid="retrying-indicator">
            {t('retrying')}
          </div>
        )}
        {(isStreaming || streamingAssistant !== null) && (
          <div aria-live="polite" className={styles.streamWrapper}>
            <ReasoningPill
              text={streamingReasoning ?? ''}
              isStreaming={isStreaming && streamingReasoning === null}
            />
            {streamingAssistant === null || streamingAssistant === '' ? (
              <div className={styles.typingRow}>
                <TypingIndicator />
              </div>
            ) : (
              <MessageBubble chatRole="assistant" streaming>
                {streamingAssistant}
              </MessageBubble>
            )}
          </div>
        )}
        {lastError !== null && (
          <div role="alert" className={styles.errorAlert}>
            {tErrors(lastError.code as ChatErrorCode, { message: lastError.message })}
          </div>
        )}
      </div>

      <div className={styles.composer}>
        <div className={styles.composerCol}>
          {stagingError !== null && (
            <div role="status" className={styles.stagingError}>
              {stagingError}
            </div>
          )}
          <ComposerAttachments items={staged} onRemove={removeStaged} />
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKeyDown}
            onPaste={onPaste}
            placeholder={t('placeholder')}
            rows={2}
            className={styles.draft}
            aria-label={t('placeholder')}
          />
          <div className={styles.hint}>{t('hint_keyboard')}</div>
        </div>
        <div className={styles.composerActions}>
          {isStreaming ? (
            <button
              type="button"
              className={styles.sendBtn}
              onClick={() => {
                cancel();
                resetScroll();
              }}
              aria-label={t('stop')}
            >
              <Icons.Stop size={14} />
              <span>{t('stop')}</span>
            </button>
          ) : (
            <button
              type="button"
              className={styles.sendBtn}
              onClick={() => void onSend()}
              disabled={!canSend}
              aria-label={t('send')}
            >
              <Icons.Send size={14} />
              <span>{t('send')}</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
