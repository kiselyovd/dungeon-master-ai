import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SCENE_ART } from '../assets/livingTabletop';
import { useStore } from '../state/useStore';

const FADE_OUT_MS = 280;
const DEBOUNCE_MS = 30_000;

type SceneTag = 'combat' | 'dialog' | 'exploration' | 'dungeon';

const TAG_VIDEO: Record<SceneTag, string> = SCENE_ART;

const KEYWORD_TAG: ReadonlyArray<readonly [RegExp, SceneTag]> = [
  [/combat|battle|fight|skirmish|ambush|боев|сраж|битв|драк/i, 'combat'],
  [/dialog|talk|conversation|parley|диалог|разговор|беседа|переговор/i, 'dialog'],
  [/dungeon|crypt|cave|cavern|tomb|catacomb|подземел|пещер|склеп|катакомб/i, 'dungeon'],
  [/explor|forest|road|wilderness|travel|journey|wood|поход|лес|путь|путеш|дорог/i, 'exploration'],
];

function pickTag(name: string): SceneTag {
  for (const [pattern, tag] of KEYWORD_TAG) {
    if (pattern.test(name)) return tag;
  }
  return 'exploration';
}

export function SceneTransitionOverlay() {
  const { t } = useTranslation('common');
  const enabled = useStore((s) => s.settings.sceneTransitionsEnabled);
  const sceneName = useStore((s) => s.session.currentScene?.name ?? null);

  const [activeTag, setActiveTag] = useState<SceneTag | null>(null);
  const [fading, setFading] = useState(false);
  const lastTriggerAt = useRef<number>(0);
  const lastSceneName = useRef<string | null>(sceneName);
  const fadeTimer = useRef<number | null>(null);
  const unmountTimer = useRef<number | null>(null);

  useEffect(() => {
    // First mount: remember the current scene without firing - we only
    // want to react to *changes* after the app is already running.
    if (lastSceneName.current === null && sceneName !== null) {
      lastSceneName.current = sceneName;
      return;
    }
    if (sceneName === lastSceneName.current) return;
    lastSceneName.current = sceneName;

    if (!enabled || sceneName === null) return;
    const now = Date.now();
    if (now - lastTriggerAt.current < DEBOUNCE_MS) return;
    lastTriggerAt.current = now;

    setActiveTag(pickTag(sceneName));
    setFading(false);
  }, [sceneName, enabled]);

  const dismiss = (): void => {
    if (activeTag === null || fading) return;
    setFading(true);
    unmountTimer.current = window.setTimeout(() => {
      setActiveTag(null);
      setFading(false);
    }, FADE_OUT_MS);
  };

  // Esc key skips an active transition. The dismiss handler reads the
  // latest activeTag/fading via closure, but we re-bind on every change
  // so the listener has fresh state at trigger time.
  useEffect(() => {
    if (activeTag === null) return;
    const onKey = (e: globalThis.KeyboardEvent): void => {
      if (e.key !== 'Escape') return;
      if (fading) return;
      setFading(true);
      unmountTimer.current = window.setTimeout(() => {
        setActiveTag(null);
        setFading(false);
      }, FADE_OUT_MS);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [activeTag, fading]);

  useEffect(() => {
    return () => {
      if (fadeTimer.current !== null) window.clearTimeout(fadeTimer.current);
      if (unmountTimer.current !== null) window.clearTimeout(unmountTimer.current);
    };
  }, []);

  if (activeTag === null) return null;

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: Esc is handled at window level above; click-to-dismiss is a convenience
    <div
      className={`dm-scene-transition${fading ? ' is-fading' : ''}`}
      role="dialog"
      aria-label={t('scene_transition_label')}
      onClick={dismiss}
    >
      <video
        className="dm-scene-transition-video"
        src={TAG_VIDEO[activeTag]}
        autoPlay
        muted
        playsInline
        onEnded={dismiss}
      >
        <track kind="captions" />
      </video>
      <button
        type="button"
        className="dm-scene-transition-skip"
        onClick={(e) => {
          e.stopPropagation();
          dismiss();
        }}
      >
        {t('skip')}
      </button>
    </div>
  );
}
