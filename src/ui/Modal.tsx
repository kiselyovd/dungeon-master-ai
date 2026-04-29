import { type ReactNode, useEffect, useId, useRef } from 'react';
import styles from './Modal.module.css';

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
}

/**
 * Accessible modal dialog primitive.
 *
 * - role="dialog" + aria-modal + aria-labelledby (auto-id title)
 * - ESC closes
 * - Focus trap inside the dialog (Tab cycles within)
 * - First tabbable element receives focus on open
 * - Returns focus to the previously-focused element on close
 * - Backdrop click closes (consumers can stop propagation if undesired)
 */
export function Modal({ open, onClose, title, children, footer }: ModalProps) {
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;

    previousFocusRef.current = document.activeElement as HTMLElement | null;

    const dialog = dialogRef.current;
    if (dialog) {
      const first = focusableElements(dialog)[0];
      if (first) {
        first.focus();
      } else {
        dialog.focus();
      }
    }

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key !== 'Tab' || !dialog) return;
      const focusables = focusableElements(dialog);
      if (focusables.length === 0) {
        e.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (!first || !last) return;
      const active = document.activeElement;
      if (e.shiftKey) {
        if (active === first || !dialog.contains(active)) {
          e.preventDefault();
          last.focus();
        }
      } else if (active === last || !dialog.contains(active)) {
        e.preventDefault();
        first.focus();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      previousFocusRef.current?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: backdrop click is a convenience dismissal; keyboard users get ESC + the explicit Cancel button
    <div
      data-testid="modal-backdrop"
      className={styles.backdrop}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        className={styles.dialog}
      >
        <h2 id={titleId} className={styles.title}>
          {title}
        </h2>
        <div>{children}</div>
        {footer && <div className={styles.footer}>{footer}</div>}
      </div>
    </div>
  );
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

function focusableElements(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute('aria-hidden'),
  );
}
