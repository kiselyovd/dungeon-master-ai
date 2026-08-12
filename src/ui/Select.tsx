import { useEffect, useId, useRef, useState } from 'react';
import { Icons } from './Icons';
import styles from './Select.module.css';

export interface SelectOption<T extends string> {
  value: T;
  label: string;
  disabled?: boolean;
}

interface SelectProps<T extends string> {
  id?: string;
  value: T;
  options: readonly SelectOption<T>[];
  onChange: (value: T) => void;
  ariaLabel?: string;
  disabled?: boolean;
  className?: string;
}

export function Select<T extends string>({
  id,
  value,
  options,
  onChange,
  ariaLabel,
  disabled = false,
  className,
}: SelectProps<T>) {
  const generatedId = useId();
  const listboxId = `${id ?? generatedId}-listbox`;
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const [open, setOpen] = useState(false);
  const selectedIndex = Math.max(
    0,
    options.findIndex((option) => option.value === value),
  );
  const [activeIndex, setActiveIndex] = useState(selectedIndex);
  const selected = options[selectedIndex];

  useEffect(() => {
    if (!open) return;
    const closeOutside = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('pointerdown', closeOutside);
    return () => document.removeEventListener('pointerdown', closeOutside);
  }, [open]);

  const closeAndFocus = () => {
    setOpen(false);
    triggerRef.current?.focus();
  };

  const move = (delta: number) => {
    if (options.length === 0) return;
    let next = activeIndex;
    for (let attempts = 0; attempts < options.length; attempts += 1) {
      next = (next + delta + options.length) % options.length;
      if (!options[next]?.disabled) break;
    }
    setActiveIndex(next);
  };

  return (
    <div ref={rootRef} className={[styles.root, className].filter(Boolean).join(' ')}>
      <button
        ref={triggerRef}
        id={id}
        type="button"
        role="combobox"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listboxId}
        aria-activedescendant={open ? `${listboxId}-option-${activeIndex}` : undefined}
        data-value={value}
        disabled={disabled}
        className={styles.trigger}
        onClick={() => {
          setActiveIndex(selectedIndex);
          setOpen((current) => !current);
        }}
        onKeyDown={(event) => {
          if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            if (!open) {
              setOpen(true);
              setActiveIndex(selectedIndex);
            } else {
              move(event.key === 'ArrowDown' ? 1 : -1);
            }
          } else if (event.key === 'Enter' || event.key === ' ') {
            if (!open) return;
            event.preventDefault();
            const option = options[activeIndex];
            if (option && !option.disabled) onChange(option.value);
            closeAndFocus();
          } else if (event.key === 'Escape' && open) {
            event.preventDefault();
            closeAndFocus();
          }
        }}
      >
        <span className={styles.value}>{selected?.label ?? ''}</span>
        <Icons.ChevronDown className={styles.chevron ?? ''} size={15} />
      </button>

      {open && (
        <div id={listboxId} role="listbox" aria-label={ariaLabel} className={styles.listbox}>
          {options.map((option, index) => (
            <button
              key={option.value}
              id={`${listboxId}-option-${index}`}
              type="button"
              role="option"
              aria-selected={option.value === value}
              data-value={option.value}
              disabled={option.disabled}
              className={styles.option}
              data-active={index === activeIndex}
              onPointerMove={() => setActiveIndex(index)}
              onClick={() => {
                onChange(option.value);
                closeAndFocus();
              }}
            >
              <span>{option.label}</span>
              {option.value === value && <Icons.Check size={15} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
