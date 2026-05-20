import {
  type ComponentPropsWithoutRef,
  createContext,
  memo,
  type ReactNode,
  useContext,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import ReactMarkdown, { type Components } from 'react-markdown';
import type { ChatRole, MessagePart } from '../state/chat';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';
import { ImageLightboxModal } from './ImageLightboxModal';
import styles from './MessageBubble.module.css';

/**
 * React context used to pass "inside a fenced <pre>" down to the <code>
 * custom component. react-markdown always wraps fenced code in <pre><code>;
 * inline code is never inside a <pre>. By setting this flag in the custom
 * <pre> component we can correctly distinguish inline code from an unlabeled
 * fenced block (both have className=undefined on the <code> element).
 */
const InsidePreContext = createContext(false);

/**
 * Custom <code> component for react-markdown.
 * Reads InsidePreContext to distinguish inline code from fenced code blocks:
 * - Inside a <pre> (fenced block, with or without language hint) -> fencedCode
 * - Outside a <pre> (inline backtick span) -> inlineCode
 */
function MdCode({ children, className: langClass }: ComponentPropsWithoutRef<'code'>) {
  const insidePre = useContext(InsidePreContext);
  return insidePre ? (
    <code className={[langClass, styles.fencedCode].filter(Boolean).join(' ')}>{children}</code>
  ) : (
    <code className={styles.inlineCode}>{children}</code>
  );
}

/**
 * Module-scope components map for ReactMarkdown. Hoisted so react-markdown
 * receives a stable reference on every render and avoids re-parsing the AST
 * when the parent re-renders. Values reference `styles.*` (a CSS-module
 * singleton frozen at module init) and `InsidePreContext` (module-level
 * constant), so module-scope hoisting is correct and zero-cost.
 */
const markdownComponents: Components = {
  h1: ({ children }) => <h1 className={styles.mdH1}>{children}</h1>,
  h2: ({ children }) => <h2 className={styles.mdH2}>{children}</h2>,
  h3: ({ children }) => <h3 className={styles.mdH3}>{children}</h3>,
  strong: ({ children }) => <strong className={styles.strong}>{children}</strong>,
  em: ({ children }) => <em className={styles.em}>{children}</em>,
  // Fenced code blocks (with OR without a language hint) are always wrapped in
  // a <pre> by react-markdown. Inline code is never inside a <pre>.
  // We set InsidePreContext here so MdCode can tell them apart without relying
  // on className (which is absent for unlabeled fenced blocks).
  pre: ({ children }) => (
    <InsidePreContext.Provider value={true}>
      <pre className={styles.pre}>{children}</pre>
    </InsidePreContext.Provider>
  ),
  code: MdCode,
  ul: ({ children }) => <ul className={styles.ul}>{children}</ul>,
  ol: ({ children }) => <ol className={styles.ol}>{children}</ol>,
  li: ({ children }) => <li className={styles.li}>{children}</li>,
};

interface MessageBubbleProps {
  chatRole: ChatRole;
  /** When true, render with a "live" treatment (italic + accent border). */
  streaming?: boolean;
  /**
   * Optional multimodal parts. When present, the bubble renders parts in
   * order (text paragraphs + inline images) instead of `children`.
   */
  parts?: MessagePart[];
  /**
   * Called when the player clicks "Retry" on a finalized assistant bubble.
   */
  onRetry?: () => void;
  /**
   * When true, the Retry button is rendered in a disabled state so the player
   * cannot trigger a second turn while one is already streaming.
   */
  retryDisabled?: boolean;
  children: ReactNode;
}

interface LightboxState {
  src: string;
  alt: string;
}

/**
 * Renders markdown for finalized assistant narration with fantasy-themed
 * custom components. Only used for non-streaming assistant bubbles; raw text
 * is intentional for streaming to avoid parse artifacts on partial tokens.
 * react-markdown is safe by default (no raw HTML passthrough).
 *
 * Wrapped in React.memo so unrelated MessageBubble re-renders (e.g. lightbox
 * state changes on a sibling) do not re-render finalized markdown bubbles.
 */
const MarkdownBody = memo(function MarkdownBody({ content }: { content: string }) {
  return (
    <div className={styles.markdownBody}>
      <ReactMarkdown components={markdownComponents}>{content}</ReactMarkdown>
    </div>
  );
});

const PREVIEW_MAX_CHARS = 120;

/**
 * Converts plain text to minimal safe HTML suitable for `entry_html` in the
 * journal store.
 *
 * Steps:
 *  1. HTML-escape special characters so no raw markup leaks.
 *  2. Split on blank-line boundaries into paragraphs; wrap each in <p>...</p>.
 *  3. Within a paragraph, convert lone newlines to <br>.
 *  4. Drop empty paragraphs.
 *
 * TODO: markdown emphasis/headings are NOT converted to rich HTML here.
 * Full markdown-to-HTML for journal entries is deferred - the spec originally
 * intended Save-to-Journal to route through the backend `journal_append` tool
 * which handles HTML conversion; B4 uses a direct slice call instead.
 */
function plainTextToEntryHtml(text: string): string {
  const escaped = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
  return escaped
    .split(/\n{2,}/)
    .map((para) => para.replace(/\n/g, '<br>').trim())
    .filter((para) => para.length > 0)
    .map((para) => `<p>${para}</p>`)
    .join('');
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max - 1).trimEnd()}...`;
}

function buildAriaPreview(parts: MessagePart[] | undefined, children: ReactNode): string {
  if (parts && parts.length > 0) {
    const text = parts
      .filter((p): p is Extract<MessagePart, { type: 'text' }> => p.type === 'text')
      .map((p) => p.text)
      .join(' ')
      .trim();
    return truncate(text, PREVIEW_MAX_CHARS);
  }
  if (typeof children === 'string') return truncate(children, PREVIEW_MAX_CHARS);
  if (typeof children === 'number') return String(children);
  return '';
}

function countImages(parts: MessagePart[] | undefined): number {
  if (!parts) return 0;
  let n = 0;
  for (const p of parts) {
    if (p.type === 'image') n += 1;
  }
  return n;
}

/**
 * Single chat-history bubble.
 *
 * Visual variants:
 * - user: gold-tinted background, right-aligned.
 * - assistant: raised background, left-aligned.
 * - assistant + streaming: italic with an accent border so the user can tell
 *   the live response apart from finalised history.
 *
 * When `parts` is provided (typically for user messages with images), the
 * bubble walks the parts and renders inline `<img>` tags alongside the text.
 * Image clicks open a fullscreen lightbox; Escape closes it.
 *
 * Accessibility: each bubble is exposed as a `role="article"` landmark with
 * an aria-label combining the role + text preview so screen readers can
 * navigate the chat history. While streaming, `aria-busy` is set so AT users
 * are notified the assistant response is still in progress.
 */
export function MessageBubble({
  chatRole,
  streaming = false,
  parts,
  onRetry,
  retryDisabled = false,
  children,
}: MessageBubbleProps) {
  const { t } = useTranslation('chat');
  const [lightbox, setLightbox] = useState<LightboxState | null>(null);

  // Narrator drop-cap fires on finalised assistant bubbles; streaming output
  // would otherwise re-trigger the cap glyph on every token, which thrashes.
  const isNarrator = chatRole === 'assistant' && !streaming;

  // Finalized assistant text goes through markdown; everything else stays raw.
  // Parts (multimodal) and streaming content always bypass markdown to avoid
  // parse artifacts on partial tokens and to keep image rendering intact.
  const renderedBody: ReactNode =
    parts && parts.length > 0 ? (
      parts.map((p, i) => renderPart(p, i, (src, alt) => setLightbox({ src, alt })))
    ) : isNarrator && typeof children === 'string' ? (
      <MarkdownBody content={children} />
    ) : (
      children
    );

  const preview = buildAriaPreview(parts, children);
  const imageCount = countImages(parts);
  const labelKey: 'bubble_user_label' | 'bubble_assistant_label' | 'bubble_system_label' =
    chatRole === 'user'
      ? 'bubble_user_label'
      : chatRole === 'system'
        ? 'bubble_system_label'
        : 'bubble_assistant_label';
  const baseLabel = t(labelKey, { preview });
  const ariaLabel =
    imageCount > 0 ? `${baseLabel} ${t('bubble_images_suffix', { count: imageCount })}` : baseLabel;

  const handleCopy = () => {
    // The '' branch is unreachable for the tray path: the tray only renders on
    // assistant bubbles, and assistant bubbles never carry parts (multimodal
    // user messages); children is always a string here.
    const text = typeof children === 'string' ? children : '';
    navigator.clipboard.writeText(text).catch(() => undefined);
  };

  const handleSaveJournal = () => {
    // Same note: the '' branch is unreachable on the tray path (see handleCopy).
    const text = typeof children === 'string' ? children : '';
    const state = useStore.getState();
    state.journal.appendEntry({
      id: crypto.randomUUID(),
      campaign_id: state.session.activeCampaignId ?? '',
      chapter: null,
      entry_html: plainTextToEntryHtml(text),
      created_at: new Date().toISOString(),
    });
  };

  return (
    <>
      <article
        className={styles.bubble}
        aria-label={ariaLabel}
        aria-busy={streaming ? true : undefined}
        data-role={chatRole}
        data-streaming={streaming ? 'true' : undefined}
        data-narrator={isNarrator ? 'true' : undefined}
        data-testid="bubble"
      >
        {renderedBody}
        {isNarrator && (
          <div className={styles.actions}>
            <button
              type="button"
              className={styles.actionBtn}
              aria-label={t('action_copy')}
              onClick={handleCopy}
            >
              <Icons.Copy size={14} />
            </button>
            <button
              type="button"
              className={styles.actionBtn}
              aria-label={t('action_retry')}
              onClick={onRetry}
              disabled={retryDisabled}
            >
              <Icons.Refresh size={14} />
            </button>
            <button
              type="button"
              className={styles.actionBtn}
              aria-label={t('action_save_journal')}
              onClick={handleSaveJournal}
            >
              <Icons.BookOpen size={14} />
            </button>
          </div>
        )}
      </article>
      {lightbox !== null && (
        <ImageLightboxModal
          src={lightbox.src}
          alt={lightbox.alt}
          onClose={() => setLightbox(null)}
        />
      )}
    </>
  );
}

function partKey(part: MessagePart, index: number): string {
  if (part.type === 'text') return `t-${index}-${part.text.length}`;
  return `i-${index}-${part.mime}-${part.data_b64.length}`;
}

function renderPart(
  part: MessagePart,
  index: number,
  onImageClick: (src: string, alt: string) => void,
): ReactNode {
  const key = partKey(part, index);
  if (part.type === 'text') {
    return (
      <p key={key} className={styles.text}>
        {part.text}
      </p>
    );
  }
  const src = `data:${part.mime};base64,${part.data_b64}`;
  const alt = part.name ?? '';
  return (
    <button
      key={key}
      type="button"
      className={styles.attachmentButton}
      onClick={() => onImageClick(src, alt)}
    >
      <img src={src} alt={alt} loading="lazy" decoding="async" className={styles.attachment} />
    </button>
  );
}
