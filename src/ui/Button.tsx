import { type ButtonHTMLAttributes, forwardRef } from 'react';

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'type'> {
  variant?: 'default' | 'primary' | 'ghost';
  /** `submit` is opt-in only - default to `button` to avoid stray form submits. */
  type?: 'button' | 'submit' | 'reset';
}

/**
 * Wraps the native button so default `type` is "button" (Biome a11y rule
 * useButtonType is satisfied automatically) and visual variants are tagged
 * via a data-attribute that CSS can target without dictating styles here.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'default', type = 'button', style, ...rest }, ref) => {
    const variantStyle = variant === 'primary' ? { borderColor: 'var(--color-accent)' } : {};
    return (
      <button
        ref={ref}
        type={type}
        data-variant={variant}
        style={{ ...variantStyle, ...style }}
        {...rest}
      />
    );
  },
);
Button.displayName = 'Button';
