import { useLayoutEffect, useRef, useState } from 'react';
import { usePresentationMotion } from '../../hooks/usePresentationMotion';
import {
  type CombatMotionFrame,
  type ConfirmedCombatEffect,
  confirmedCombatEffects,
} from './model/confirmedMotion';

/** Consume background/reduced projections as baselines, never deferred effects. */
export function useConfirmedCombatMotion(frame: CombatMotionFrame) {
  const previous = useRef<CombatMotionFrame | null>(null);
  const [effects, setEffects] = useState<ConfirmedCombatEffect[]>([]);
  const motion = usePresentationMotion();
  const enabled = motion.active && !motion.reduced;

  useLayoutEffect(() => {
    const observed = previous.current;
    if (
      observed &&
      observed.scopeKey === frame.scopeKey &&
      observed.ownActorId === frame.ownActorId &&
      frame.revision <= observed.revision
    ) {
      setEffects([]);
      return;
    }
    const next = enabled ? confirmedCombatEffects(previous.current, frame) : [];
    previous.current = frame;
    setEffects(next);
    if (!next.length) return;
    if (import.meta.env.DEV)
      console.debug('[combat.motion]', {
        revision: frame.revision,
        effects: next.map((effect) => effect.kind),
      });
    const timer = setTimeout(() => setEffects([]), 280);
    return () => clearTimeout(timer);
  }, [frame, enabled]);

  return effects;
}
