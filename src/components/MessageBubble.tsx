import { type ReactNode, useState } from 'react';
import type { ChatRole, MessagePart } from '../state/chat';
import { ImageLightboxModal } from './ImageLightboxModal';
import styles from './MessageBubble.module.css';

interface MessageBubbleProps {
  chatRole: ChatRole;
  /** When true, render with a "live" treatment (italic + accent border). */
  streaming?: boolean;
  /**
   * Optional multimodal parts. When present, the bubble renders parts in
   * order (text paragraphs + inline images) instead of `children`.
   */
  parts?: MessagePart[];
  children: ReactNode;
}

interface LightboxState {
  src: string;
  alt: string;
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
 */
export function MessageBubble({
  chatRole,
  streaming = false,
  parts,
  children,
}: MessageBubbleProps) {
  const [lightbox, setLightbox] = useState<LightboxState | null>(null);

  // Narrator drop-cap fires on finalised assistant bubbles; streaming output
  // would otherwise re-trigger the cap glyph on every token, which thrashes.
  const isNarrator = chatRole === 'assistant' && !streaming;
  const renderedBody =
    parts && parts.length > 0
      ? parts.map((p, i) => renderPart(p, i, (src, alt) => setLightbox({ src, alt })))
      : children;
  return (
    <>
      <div
        className={styles.bubble}
        data-role={chatRole}
        data-streaming={streaming ? 'true' : undefined}
        data-narrator={isNarrator ? 'true' : undefined}
        data-testid="bubble"
      >
        {renderedBody}
      </div>
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

function renderPart(
  part: MessagePart,
  index: number,
  onImageClick: (src: string, alt: string) => void,
): ReactNode {
  if (part.type === 'text') {
    return (
      // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
      <p key={index} className={styles.text}>
        {part.text}
      </p>
    );
  }
  const src = `data:${part.mime};base64,${part.data_b64}`;
  const alt = part.name ?? '';
  return (
    <img
      // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
      key={index}
      src={src}
      alt={alt}
      loading="lazy"
      decoding="async"
      className={styles.attachment}
      onClick={() => onImageClick(src, alt)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onImageClick(src, alt);
        }
      }}
    />
  );
}
