import type { ReactNode } from 'react';

export function ContextSurface({
  embedded,
  label,
  className,
  state,
  onEscape,
  children,
}: {
  embedded: boolean;
  label: string;
  className: string | undefined;
  state: 'open' | 'closing';
  onEscape: () => void;
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
