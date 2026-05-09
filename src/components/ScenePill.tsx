import { useTranslation } from 'react-i18next';
import type { CurrentScene } from '../state/session';
import { Icons } from '../ui/Icons';

interface ScenePillProps {
  scene: CurrentScene | null;
}

/**
 * Titlebar-centre pill that surfaces the active scene name + step counter.
 *
 * Renders nothing when no scene is set so the centre slot stays empty
 * during the no-campaign / startup state. When set, the pill is announced
 * to assistive tech via `role="status" aria-live="polite"` so screen
 * readers pick up the scene transition without stealing focus.
 */
export function ScenePill({ scene }: ScenePillProps) {
  const { t } = useTranslation('common');
  if (scene === null) return null;
  return (
    <div className="dm-scene-pill" role="status" aria-live="polite">
      <Icons.Compass size={12} />
      <span>{scene.name}</span>
      <span className="dm-scene-pill-step dm-mono">
        {'· '}
        {t('scene_step', { count: scene.stepCounter })}
      </span>
    </div>
  );
}
