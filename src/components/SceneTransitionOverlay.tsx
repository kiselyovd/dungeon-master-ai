import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SCENE_ART } from '../assets/livingTabletop';
import { useStore } from '../state/useStore';

const FADE_OUT_MS = 280;
const DISPLAY_MS = 3_200;
const DEBOUNCE_MS = 30_000;

type SceneTag = 'combat' | 'dialog' | 'exploration' | 'dungeon';

const TAG_ART: Record<SceneTag, string> = SCENE_ART;

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
  const didMount = useRef(false);
  const fadeTimer = useRef<number | null>(null);
  const unmountTimer = useRef<number | null>(null);

  useEffect(() => {
    // First render only: remember hydrated state without firing. A later
    // null -> scene change is a real transition and must remain visible.
    if (!didMount.current) {
      didMount.current = true;
      lastSceneName.current = sceneName;
      return;
    }
    if (sceneName === lastSceneName.current) return;
    lastSceneName.current = sceneName;

    if (!enabled || sceneName === null) return;
    const now = Date.now();
    if (now - lastTriggerAt.current < DEBOUNCE_MS) return;
    lastTriggerAt.current = now;

    if (fadeTimer.current !== null) window.clearTimeout(fadeTimer.current);
    if (unmountTimer.current !== null) window.clearTimeout(unmountTimer.current);
    setActiveTag(pickTag(sceneName));
    setFading(false);
  }, [sceneName, enabled]);

  const dismiss = (): void => {
    if (activeTag === null || fading) return;
    if (fadeTimer.current !== null) {
      window.clearTimeout(fadeTimer.current);
      fadeTimer.current = null;
    }
    if (unmountTimer.current !== null) window.clearTimeout(unmountTimer.current);
    setFading(true);
    unmountTimer.current = window.setTimeout(() => {
      setActiveTag(null);
      setFading(false);
      unmountTimer.current = null;
    }, FADE_OUT_MS);
  };

  useEffect(() => {
    if (activeTag === null) return;
    fadeTimer.current = window.setTimeout(() => {
      fadeTimer.current = null;
      setFading(true);
      unmountTimer.current = window.setTimeout(() => {
        setActiveTag(null);
        setFading(false);
        unmountTimer.current = null;
      }, FADE_OUT_MS);
    }, DISPLAY_MS);
  }, [activeTag]);

  // Esc key skips an active transition. The dismiss handler reads the
  // latest activeTag/fading via closure, but we re-bind on every change
  // so the listener has fresh state at trigger time.
  useEffect(() => {
    if (activeTag === null) return;
    const onKey = (e: globalThis.KeyboardEvent): void => {
      if (e.key !== 'Escape') return;
      if (fading) return;
      if (fadeTimer.current !== null) {
        window.clearTimeout(fadeTimer.current);
        fadeTimer.current = null;
      }
      if (unmountTimer.current !== null) window.clearTimeout(unmountTimer.current);
      setFading(true);
      unmountTimer.current = window.setTimeout(() => {
        setActiveTag(null);
        setFading(false);
        unmountTimer.current = null;
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
      <div className="dm-scene-transition-art" data-art-direction="living-tabletop">
        <img
          className="dm-scene-transition-image"
          src={TAG_ART[activeTag]}
          alt=""
          data-testid="scene-transition-art"
        />
        <span className="dm-ambient-light" aria-hidden="true" />
        <span className="dm-ambient-dust" aria-hidden="true" />
      </div>
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
