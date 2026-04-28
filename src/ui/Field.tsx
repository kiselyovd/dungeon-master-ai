import { type ReactNode, useId } from 'react';

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
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-1)' }}>
      <label
        htmlFor={inputId}
        style={{
          fontSize: 'var(--text-sm)',
          color: 'var(--color-fg-secondary)',
        }}
      >
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
          style={{
            fontSize: 'var(--text-xs)',
            color: error ? 'var(--color-danger)' : 'var(--color-fg-muted)',
          }}
        >
          {message}
        </div>
      )}
    </div>
  );
}
