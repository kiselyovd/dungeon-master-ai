import type { ReactNode } from 'react';

export function ContextSurface({
  embedded,
  label,
  className,
  state,
  onEscape,
  onBackdrop,
  children,
}: {
  embedded: boolean;
  label: string;
  className: string | undefined;
  state: 'open' | 'closing';
  onEscape: () => void;
  onBackdrop?: (() => void) | undefined;
  children: ReactNode;
}) {
  if (embedded)
    return (
      <section className={className} aria-label={label}>
        {children}
      </section>
    );
  return (
    <div
      className={className}
      data-state={state}
      role="dialog"
      aria-modal="true"
      aria-label={label}
      onClick={(event) => {
        if (event.target === event.currentTarget) onBackdrop?.();
      }}
      onKeyDown={(event) => {
        if (
          event.key === 'Escape' &&
          event.target instanceof Element &&
          event.target.closest('[role="dialog"]') === event.currentTarget
        )
          onEscape();
      }}
    >
      {children}
    </div>
  );
}
