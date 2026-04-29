import { type ReactNode, useId } from 'react';
import styles from './Field.module.css';

interface FieldProps {
  label: ReactNode;
  /** Optional inline error rendered below the input. */
  error?: ReactNode;
  /** Optional helper text rendered below the input when there's no error. */
  helper?: ReactNode;
  /**
   * Render-prop receives the auto-generated id; the consumer must spread it
   * onto the input element so `<label htmlFor>` and aria-describedby work.
   */
  children: (props: {
    id: string;
    'aria-invalid'?: boolean;
    'aria-describedby'?: string;
  }) => ReactNode;
}

export function Field({ label, error, helper, children }: FieldProps) {
  const inputId = useId();
  const messageId = useId();
  const message = error ?? helper;

  return (
    <div className={styles.root}>
      <label htmlFor={inputId} className={styles.label}>
        {label}
      </label>
      {children({
        id: inputId,
        ...(error ? { 'aria-invalid': true } : {}),
        ...(message ? { 'aria-describedby': messageId } : {}),
      })}
      {message && (
        <div
          id={messageId}
          role={error ? 'alert' : undefined}
          className={styles.message}
          data-error={error ? 'true' : undefined}
        >
          {message}
        </div>
      )}
    </div>
  );
}
