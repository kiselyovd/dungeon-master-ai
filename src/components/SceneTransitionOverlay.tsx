import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SCENE_ART, SCENE_ART_IDS } from '../assets/livingTabletop';
import { usePresentationMotion } from '../hooks/usePresentationMotion';
import { useStore } from '../state/useStore';
import { Button } from '../ui/Button';

interface Props {
  scopeKey: string;
  active: boolean;
  encounterId: string | null;
  art?: { id: string; uri: string } | null;
}
const DISPLAY_MS = 3_200;
type Moment = { id: string; kind: 'combat_enter' | 'combat_exit' };

/** Optional canonical major-event accent. Ordinary movement never creates it.
 * It lives inside the table, does not take focus and permits pointer passthrough. */
export function SceneTransitionOverlay({ scopeKey, active, encounterId, art }: Props) {
  const { t } = useTranslation('common');
  const enabled = useStore((state) => state.settings.sceneTransitionsEnabled);
  const motion = usePresentationMotion();
  const previous = useRef({ scopeKey, active, encounterId });
  const didMount = useRef(false);
  const seen = useRef(new Set<string>());
  const [moment, setMoment] = useState<Moment | null>(null);

  useEffect(() => {
    const old = previous.current;
    previous.current = { scopeKey, active, encounterId };
    if (!didMount.current || old.scopeKey !== scopeKey) {
      didMount.current = true;
      seen.current.clear();
      if (active && encounterId) seen.current.add(`${encounterId}:combat_enter`);
      setMoment(null);
      return;
    }
    if (old.active === active) {
      if (!enabled || !motion.active || motion.reduced) setMoment(null);
      return;
    }
    const id = active ? encounterId : old.encounterId;
    if (!id) return;
    const kind = active ? 'combat_enter' : 'combat_exit';
    const key = `${id}:${kind}`;
    if (seen.current.has(key)) return;
    seen.current.add(key);
    if (seen.current.size > 32) seen.current.delete(seen.current.values().next().value ?? '');
    if (!enabled || !motion.active || motion.reduced) {
      setMoment(null);
      return;
    }
    setMoment({ id: JSON.stringify([scopeKey, key]), kind });
    if (import.meta.env.DEV) console.debug('[scene.motion]', { kind, result: 'shown' });
  }, [scopeKey, active, encounterId, enabled, motion.active, motion.reduced]);

  useEffect(() => {
    if (!moment) return;
    const timer = setTimeout(() => setMoment(null), DISPLAY_MS);
    return () => clearTimeout(timer);
  }, [moment]);

  if (!moment) return null;
  return (
    <aside
      className="dm-scene-transition"
      role="status"
      aria-live="polite"
      data-moment-id={moment.id}
    >
      <img
        className="dm-scene-transition-image"
        src={art?.uri ?? SCENE_ART.combat}
        alt=""
        data-testid="scene-transition-art"
        data-asset-id={art?.id ?? SCENE_ART_IDS.combat}
      />
      <div className="dm-scene-transition-caption">
        <strong>
          {t(moment.kind === 'combat_enter' ? 'cinematic_combat_enter' : 'cinematic_combat_exit')}
        </strong>
        <Button className="dm-scene-transition-skip" onClick={() => setMoment(null)}>
          {t('skip')}
        </Button>
      </div>
    </aside>
  );
}
