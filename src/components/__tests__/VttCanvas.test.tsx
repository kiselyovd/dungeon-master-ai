import { act, render } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @pixi/react to avoid WebGL in jsdom.
vi.mock('@pixi/react', () => ({
  Application: ({ children, ...props }: { children?: ReactNode } & Record<string, unknown>) => (
    <div data-testid="pixi-app" data-props={JSON.stringify(props)}>
      {children}
    </div>
  ),
  extend: () => undefined,
}));

vi.mock('pixi.js', () => ({
  Container: class {},
  Graphics: class {},
}));

import { VttCanvas } from '../VttCanvas';

type ResizeCallback = (entries: ResizeObserverEntry[]) => void;

interface FakeResizeObserver {
  observe: (target: Element) => void;
  unobserve: (target: Element) => void;
  disconnect: () => void;
}

const observers: { cb: ResizeCallback; targets: Element[] }[] = [];

function installResizeObserverMock(): void {
  observers.length = 0;
  globalThis.ResizeObserver = class implements FakeResizeObserver {
    private readonly targets: Element[] = [];
    constructor(cb: ResizeCallback) {
      observers.push({ cb, targets: this.targets });
    }
    observe(target: Element): void {
      this.targets.push(target);
    }
    unobserve(target: Element): void {
      const idx = this.targets.indexOf(target);
      if (idx >= 0) this.targets.splice(idx, 1);
    }
    disconnect(): void {
      this.targets.length = 0;
    }
  } as unknown as typeof ResizeObserver;
}

function fireResize(width: number, height: number): void {
  for (const { cb, targets } of observers) {
    const target = targets[0];
    if (!target) continue;
    const entry = {
      target,
      contentRect: {
        width,
        height,
        top: 0,
        left: 0,
        bottom: height,
        right: width,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      } as DOMRectReadOnly,
      borderBoxSize: [],
      contentBoxSize: [],
      devicePixelContentBoxSize: [],
    } as unknown as ResizeObserverEntry;
    cb([entry]);
  }
}

function flushRaf(): void {
  // VttCanvas batches resize updates with requestAnimationFrame; jsdom does
  // not auto-tick rAF, so push the test clock far enough that rAF callbacks
  // fire (vitest's fake timers covers rAF when used, but a microtask flush
  // works because vitest provides a polyfilled rAF that runs on a 0ms timer).
  act(() => {
    vi.advanceTimersByTime(20);
  });
}

describe('VttCanvas', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    installResizeObserverMock();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('mounts with the Pixi Application', () => {
    const { getByTestId } = render(<VttCanvas />);
    const app = getByTestId('pixi-app');
    expect(app).toBeInTheDocument();
  });

  it('clamps to the minimum canvas size when no container box is available', () => {
    // In jsdom clientWidth/clientHeight are 0 and ResizeObserver does not
    // auto-fire on mount, so the canvas falls back to the MIN_CANVAS_PX floor
    // (60px) on both axes. The widthCells/cellSize props now affect only the
    // grid-line count, not the Pixi viewport size.
    const { getByTestId } = render(<VttCanvas widthCells={10} heightCells={15} cellSize={32} />);
    const app = getByTestId('pixi-app');
    const props = JSON.parse(app.getAttribute('data-props') ?? '{}') as Record<string, unknown>;
    expect(props.width).toBe(60);
    expect(props.height).toBe(60);
  });

  it('renders the empty-state overlay when there are no tokens', () => {
    const { container } = render(<VttCanvas />);
    const empty = container.querySelector('.dm-vtt-empty');
    expect(empty).toBeInTheDocument();
    // Container's title node carries the translated/key text; assert it is non-empty.
    const title = container.querySelector('.dm-vtt-empty-title');
    expect(title?.textContent ?? '').not.toBe('');
  });

  it('makes the Pixi canvas fill the container exactly on resize', () => {
    const { getByTestId } = render(<VttCanvas cellSize={30} />);

    act(() => {
      fireResize(900, 450);
    });
    flushRaf();

    const app = getByTestId('pixi-app');
    const props = JSON.parse(app.getAttribute('data-props') ?? '{}') as Record<string, unknown>;
    // Canvas now fills the container (no centering, no cell-aligned crop).
    expect(props.width).toBe(900);
    expect(props.height).toBe(450);
  });

  it('fills a 600x500 container exactly (regression: prevent dark margins)', () => {
    const { getByTestId } = render(<VttCanvas cellSize={30} />);

    act(() => {
      fireResize(600, 500);
    });
    flushRaf();

    const app = getByTestId('pixi-app');
    const props = JSON.parse(app.getAttribute('data-props') ?? '{}') as Record<string, unknown>;
    // 600 is a clean multiple of 30 but 500 is not - canvas must still be
    // 600x500 (NOT 600x480 = floor(500/30) * 30) so the grid background fills
    // the full pane and no dark strip shows above/below the cell-aligned grid.
    expect(props.width).toBe(600);
    expect(props.height).toBe(500);
  });

  it('clamps to a minimum size when the container is smaller than the floor', () => {
    const { getByTestId } = render(<VttCanvas cellSize={30} />);

    act(() => {
      fireResize(20, 20);
    });
    flushRaf();

    const app = getByTestId('pixi-app');
    const props = JSON.parse(app.getAttribute('data-props') ?? '{}') as Record<string, unknown>;
    // MIN_CANVAS_PX floor is 60px on both axes - well below typical UI sizes
    // so a narrow chat resize never strands the canvas at a misleading size.
    expect(props.width).toBe(60);
    expect(props.height).toBe(60);
  });

  it('observes the .dm-vtt root element', () => {
    render(<VttCanvas />);
    expect(observers.length).toBeGreaterThan(0);
    const observer = observers[0];
    expect(observer).toBeDefined();
    expect(observer?.targets[0]).toBeInstanceOf(HTMLElement);
    expect((observer?.targets[0] as HTMLElement).classList.contains('dm-vtt')).toBe(true);
  });
});
