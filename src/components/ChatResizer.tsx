import {
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useEffect,
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
  // Stored so we can detach without recreating closures.
  cancelHandler: () => void;
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

  /**
   * End an in-flight drag. When `cancel` is true, revert to the width captured
   * at pointerdown (used for Escape, window blur, and tab visibility loss);
   * otherwise commit the final pointer position. Either way, we always tear
   * down the body class, window listeners, and dragRef so the component
   * cannot leak global cursor state.
   */
  const finishDrag = useCallback(
    (clientX: number | null, cancel: boolean) => {
      const drag = dragRef.current;
      if (drag === null) return;
      let next: number;
      if (cancel || clientX === null) {
        next = drag.startWidth;
      } else {
        const delta = drag.startClientX - clientX;
        next = clamp(drag.startWidth + delta);
      }
      window.removeEventListener('blur', drag.cancelHandler);
      document.removeEventListener('visibilitychange', drag.cancelHandler);
      dragRef.current = null;
      setIsDragging(false);
      document.body.classList.remove(RESIZING_BODY_CLASS);
      applyWidth(next);
      setChatPanelWidth(next);
    },
    [applyWidth, setChatPanelWidth],
  );

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLButtonElement>) => {
      // Only respond to the primary button; ignore right/middle clicks.
      if (event.button !== 0) return;
      event.preventDefault();
      const target = event.currentTarget;
      target.setPointerCapture(event.pointerId);
      // Build the cancel handler eagerly so blur/visibilitychange callbacks
      // share the same identity used to attach + detach.
      const cancelHandler = () => {
        finishDrag(null, true);
      };
      dragRef.current = {
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startWidth: chatPanelWidth,
        cancelHandler,
      };
      setIsDragging(true);
      document.body.classList.add(RESIZING_BODY_CLASS);
      // OS-level focus loss (Alt+Tab, native dialog, devtools popout) does
      // not always emit pointercancel; treat blur/visibility loss as a
      // cancel so the body class never sticks.
      window.addEventListener('blur', cancelHandler);
      document.addEventListener('visibilitychange', cancelHandler);
    },
    [chatPanelWidth, finishDrag],
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

  const onPointerEnd = useCallback(
    (event: ReactPointerEvent<HTMLButtonElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) return;
      try {
        event.currentTarget.releasePointerCapture(event.pointerId);
      } catch {
        // setPointerCapture may have been auto-released already; ignore.
      }
      const cancel = event.type === 'pointercancel';
      finishDrag(event.clientX, cancel);
    },
    [finishDrag],
  );

  const onKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLButtonElement>) => {
      // Escape during an active drag reverts to the pre-drag width. When
      // there is no drag in flight, Escape is a no-op (keyboard stepping
      // commits each keystroke; there is nothing to revert to).
      if (event.key === 'Escape') {
        if (dragRef.current !== null) {
          event.preventDefault();
          finishDrag(null, true);
        }
        return;
      }
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
    [applyWidth, chatPanelWidth, finishDrag, setChatPanelWidth],
  );

  // Unmount cleanup. Without this, an unmount mid-drag (route change, error
  // boundary, dev hot reload) would leave `dm-chat-resizing` on <body> and
  // freeze the global cursor until the next page reload. We also detach any
  // window listeners that the in-flight drag attached.
  useEffect(
    () => () => {
      const drag = dragRef.current;
      if (drag !== null) {
        window.removeEventListener('blur', drag.cancelHandler);
        document.removeEventListener('visibilitychange', drag.cancelHandler);
        dragRef.current = null;
      }
      document.body.classList.remove(RESIZING_BODY_CLASS);
    },
    [],
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
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      onKeyDown={onKeyDown}
    />
  );
}
