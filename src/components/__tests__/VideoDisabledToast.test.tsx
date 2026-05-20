import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { VideoDisabledToast } from '../VideoDisabledToast';

/**
 * VideoDisabledToast - 5 behaviors:
 *
 * 1. Once-per-scene-change contract: no toast on initial mount; toast on scene
 *    change; same scene name twice does not re-trigger.
 * 2. When videoEnabled=true or sceneTransitionsEnabled=false, a scene change
 *    must NOT show the toast.
 * 3. Auto-dismisses after 8 seconds following a scene change.
 * 4. "Enable video" button calls the onOpenVideoSettings prop.
 * 5. "Dismiss" button hides the toast immediately.
 */

/** Helper: rerender inside act so effects flush synchronously. */
async function rerenderAct(
  rerender: (ui: React.ReactElement) => void,
  ui: React.ReactElement,
): Promise<void> {
  await act(async () => {
    rerender(ui);
  });
}

describe('VideoDisabledToast', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('once-per-scene-change: no toast on mount, visible on scene change, no re-trigger for same scene', async () => {
    const { rerender } = render(
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="combat"
        onOpenVideoSettings={() => {}}
      />,
    );

    // Initial mount with sceneName already set - toast must NOT appear.
    expect(screen.queryByRole('status')).not.toBeInTheDocument();

    // Genuine scene change -> toast must appear.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="dungeon"
        onOpenVideoSettings={() => {}}
      />,
    );
    expect(screen.getByRole('status')).toBeInTheDocument();

    // Dismiss so we get a clean "not visible" baseline.
    await userEvent.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();

    // Same scene name again - must not reappear.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="dungeon"
        onOpenVideoSettings={() => {}}
      />,
    );
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('does not show when videoEnabled is true or sceneTransitionsEnabled is false', async () => {
    const { rerender } = render(
      <VideoDisabledToast
        videoEnabled={true}
        sceneTransitionsEnabled={true}
        sceneName="combat"
        onOpenVideoSettings={() => {}}
      />,
    );

    // Scene change with videoEnabled=true - must not trigger.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={true}
        sceneTransitionsEnabled={true}
        sceneName="dungeon"
        onOpenVideoSettings={() => {}}
      />,
    );
    expect(screen.queryByRole('status')).not.toBeInTheDocument();

    // Scene change with sceneTransitionsEnabled=false - must not trigger.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={false}
        sceneName="exploration"
        onOpenVideoSettings={() => {}}
      />,
    );
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('auto-dismisses after 8 seconds following a scene change', async () => {
    vi.useFakeTimers();
    const { rerender } = render(
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="combat"
        onOpenVideoSettings={() => {}}
      />,
    );

    // Trigger via genuine scene change.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="dungeon"
        onOpenVideoSettings={() => {}}
      />,
    );

    expect(screen.getByRole('status')).toBeInTheDocument();

    // Advance 8 seconds - toast must auto-dismiss.
    await act(async () => {
      vi.advanceTimersByTime(8000);
    });

    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('"Enable video" button calls onOpenVideoSettings after a scene change', async () => {
    const onOpenVideoSettings = vi.fn();
    const { rerender } = render(
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="combat"
        onOpenVideoSettings={onOpenVideoSettings}
      />,
    );

    // Trigger via genuine scene change.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="dungeon"
        onOpenVideoSettings={onOpenVideoSettings}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /enable video/i }));
    expect(onOpenVideoSettings).toHaveBeenCalledOnce();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('"Dismiss" button hides the toast immediately after a scene change', async () => {
    const { rerender } = render(
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="combat"
        onOpenVideoSettings={() => {}}
      />,
    );

    // Trigger via genuine scene change.
    await rerenderAct(
      rerender,
      <VideoDisabledToast
        videoEnabled={false}
        sceneTransitionsEnabled={true}
        sceneName="exploration"
        onOpenVideoSettings={() => {}}
      />,
    );

    expect(screen.getByRole('status')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /dismiss/i }));

    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });
});
