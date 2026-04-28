import { render } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

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

describe('VttCanvas', () => {
  it('mounts with default 20x20 grid', () => {
    const { getByTestId } = render(<VttCanvas />);
    const app = getByTestId('pixi-app');
    expect(app).toBeInTheDocument();
  });

  it('accepts custom grid size', () => {
    const { getByTestId } = render(<VttCanvas widthCells={10} heightCells={15} cellSize={32} />);
    const app = getByTestId('pixi-app');
    const props = JSON.parse(app.getAttribute('data-props') ?? '{}') as Record<string, unknown>;
    expect(props.width).toBe(320);
    expect(props.height).toBe(480);
  });
});
