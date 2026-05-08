import type { RuntimeState } from '../state/localMode';
import styles from './RuntimeStatusPill.module.css';

interface Props {
  label: string;
  state: RuntimeState;
}

export function RuntimeStatusPill({ label, state }: Props) {
  const className =
    state.state === 'ready'
      ? styles.ready
      : state.state === 'failed'
        ? styles.failed
        : state.state === 'starting'
          ? styles.starting
          : styles.off;
  const text =
    state.state === 'failed'
      ? state.reason
      : state.state === 'ready'
        ? `ready :${state.port}`
        : state.state;

  return (
    <span className={`${styles.pill} ${className}`}>
      <strong>{label}</strong> {text}
    </span>
  );
}
