import {
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { MAX_CHAT_WIDTH, MIN_CHAT_WIDTH } from '../state/settings';
import { useStore } from '../state/useStore';
import styles from './ChatResizer.module.css';

const KEYBOARD_STEP_PX = 16;
const KEYBOARD_STEP_LARGE_PX = 64;
const RESIZING_BODY_CLASS = 'dm-chat-resizing';

function clamp(width: number): number {
  if (!Number.isFinite(width)) return MIN_CHAT_WIDTH;
  if (width < MIN_CHAT_WIDTH) return MIN_CHAT_WIDTH;
  if (width > MAX_CHAT_WIDTH) return MAX_CHAT_WIDTH;
  return width;
}

interface DragSession {
  pointerId: number;
  startClientX: number;
  startWidth: number;
}

/**
 * Drag handle on the left border of the chat panel. While dragging, we
 * mutate the `--chat-width` CSS variable directly on `document.documentElement`
 * so the grid track resizes without a Zustand-driven re-render on every
 * mousemove. On pointerup we commit the final width to the settings slice,
 * which the persist middleware then writes through to settings.json.
 */
export function ChatResizer() {
  const { t } = useTranslation('chat');
  const chatPanelWidth = useStore((s) => s.settings.chatPanelWidth);
  const setChatPanelWidth = useStore((s) => s.settings.setChatPanelWidth);
  const dragRef = useRef<DragSession | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  const applyWidth = useCallback((width: number) => {
    document.documentElement.style.setProperty('--chat-width', `${width}px`);
  }, []);

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLButtonElement>) => {
      // Only respond to the primary button; ignore right/middle clicks.
      if (event.button !== 0) return;
      event.preventDefault();
      const target = event.currentTarget;
      target.setPointerCapture(event.pointerId);
      dragRef.current = {
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startWidth: chatPanelWidth,
      };
      setIsDragging(true);
      document.body.classList.add(RESIZING_BODY_CLASS);
    },
    [chatPanelWidth],
  );

  const onPointerMove = useCallback(
    (event: ReactPointerEvent<HTMLButtonElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) return;
      // Dragging LEFT widens the chat (handle sits on the left border, so
      // moving it leftward increases panel width).
      const delta = drag.startClientX - event.clientX;
      const next = clamp(drag.startWidth + delta);
      applyWidth(next);
    },
    [applyWidth],
  );

  const finishDrag = useCallback(
    (event: ReactPointerEvent<HTMLButtonElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) return;
      const delta = drag.startClientX - event.clientX;
      const next = clamp(drag.startWidth + delta);
      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {
        // setPointerCapture may have been auto-released already; ignore.
      }
      dragRef.current = null;
      setIsDragging(false);
      document.body.classList.remove(RESIZING_BODY_CLASS);
      applyWidth(next);
      setChatPanelWidth(next);
    },
    [applyWidth, setChatPanelWidth],
  );

  const onKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLButtonElement>) => {
      const step = event.shiftKey ? KEYBOARD_STEP_LARGE_PX : KEYBOARD_STEP_PX;
      let next: number | null = null;
      if (event.key === 'ArrowLeft') {
        // Left arrow widens the chat (mirrors drag direction).
        next = clamp(chatPanelWidth + step);
      } else if (event.key === 'ArrowRight') {
        next = clamp(chatPanelWidth - step);
      } else if (event.key === 'Home') {
        next = MIN_CHAT_WIDTH;
      } else if (event.key === 'End') {
        next = MAX_CHAT_WIDTH;
      }
      if (next !== null) {
        event.preventDefault();
        applyWidth(next);
        setChatPanelWidth(next);
      }
    },
    [applyWidth, chatPanelWidth, setChatPanelWidth],
  );

  const label = t('resize_handle_label');

  return (
    // biome-ignore lint/a11y/useSemanticElements: an <hr> cannot host pointer/keyboard handlers nor receive focus, but the WAI-ARIA "separator" role on a focusable <button> is the standard pattern for an interactive resize handle (see WAI Resize Handle pattern). The button has type="button" so it doesn't submit forms.
    <button
      type="button"
      className={`${styles.handle} ${isDragging ? styles.dragging : ''}`}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={MIN_CHAT_WIDTH}
      aria-valuemax={MAX_CHAT_WIDTH}
      aria-valuenow={chatPanelWidth}
      title={label}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={finishDrag}
      onPointerCancel={finishDrag}
      onKeyDown={onKeyDown}
    />
  );
}
