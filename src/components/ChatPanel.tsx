import {
  type ClipboardEvent as ReactClipboardEvent,
  type DragEvent as ReactDragEvent,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import type { ChatErrorCode } from '../api/errors';
import { useChat } from '../hooks/useChat';
import { useStickyScroll } from '../hooks/useStickyScroll';
import { fileToDataUrl } from '../lib/fileToDataUrl';
import type { StagedImage } from '../state/chat';
import styles from './ChatPanel.module.css';
import { ComposerAttachments } from './ComposerAttachments';
import { MessageBubble } from './MessageBubble';
import { TypingIndicator } from './TypingIndicator';

const VALID_IMAGE_MIMES = new Set(['image/png', 'image/jpeg', 'image/webp']);
const MAX_IMAGE_BYTES = 5 * 1024 * 1024;
const MAX_IMAGES_PER_MESSAGE = 4;

export function ChatPanel() {
  const { t } = useTranslation('chat');
  const { t: tErrors } = useTranslation('errors');
  const { messages, streamingAssistant, isStreaming, lastError, send, cancel } = useChat();
  const [draft, setDraft] = useState('');
  const [staged, setStaged] = useState<StagedImage[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [stagingError, setStagingError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const { ref: historyRef, onScroll, scrollToBottom } = useStickyScroll(100);

  // biome-ignore lint/correctness/useExhaustiveDependencies: scrollToBottom intentionally re-fires only when conversation length changes
  useEffect(() => {
    scrollToBottom();
  }, [messages.length, streamingAssistant, scrollToBottom]);

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

  const canSend = !isStreaming && (draft.trim().length > 0 || staged.length > 0);

  const onSend = async () => {
    if (!canSend) return;
    const text = draft;
    const images = staged;
    setDraft('');
    setStaged([]);
    setStagingError(null);
    await send(text, images);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void onSend();
    } else if (e.key === 'Escape' && isStreaming) {
      e.preventDefault();
      cancel();
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
    if (e.dataTransfer.files.length > 0) void addFiles(e.dataTransfer.files);
  };

  // Window-level ESC also aborts an in-flight stream so the user can hit it
  // even when focus has wandered out of the textarea.
  useEffect(() => {
    if (!isStreaming) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isStreaming, cancel]);

  return (
    <div
      className={`${styles.panel} ${isDragging ? styles.dragging : ''}`}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      <div ref={historyRef} className={styles.history} onScroll={onScroll}>
        {messages.map((m) =>
          m.parts !== undefined ? (
            <MessageBubble key={m.id} chatRole={m.role} parts={m.parts}>
              {m.content}
            </MessageBubble>
          ) : (
            <MessageBubble key={m.id} chatRole={m.role}>
              {m.content}
            </MessageBubble>
          ),
        )}
        {(isStreaming || streamingAssistant !== null) && (
          <div aria-live="polite" className={styles.streamWrapper}>
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
        <input
          ref={fileInputRef}
          type="file"
          accept="image/png,image/jpeg,image/webp"
          multiple
          hidden
          onChange={(e) => {
            if (e.target.files) void addFiles(e.target.files);
            e.target.value = '';
          }}
        />
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
          />
        </div>
        <div className={styles.composerActions}>
          <button
            type="button"
            aria-label={t('attach_image')}
            className={styles.paperclip}
            onClick={() => fileInputRef.current?.click()}
            disabled={isStreaming || staged.length >= MAX_IMAGES_PER_MESSAGE}
          >
            +
          </button>
          {isStreaming ? (
            <button type="button" onClick={cancel} aria-label={t('stop')}>
              {t('stop')}
            </button>
          ) : (
            <button type="button" onClick={() => void onSend()} disabled={!canSend}>
              {t('send')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
