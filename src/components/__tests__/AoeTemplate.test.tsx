import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AoeTemplate } from '../AoeTemplate';

describe('AoeTemplate', () => {
  it('renders cone shape', () => {
    const { getByTestId } = render(
      <AoeTemplate shape="cone" originX={100} originY={100} cellSize={30} sizeInFt={15} />,
    );
    expect(getByTestId('aoe-cone')).toBeTruthy();
  });

  it('renders sphere shape', () => {
    const { getByTestId } = render(
      <AoeTemplate shape="sphere" originX={200} originY={200} cellSize={30} sizeInFt={20} />,
    );
    expect(getByTestId('aoe-sphere')).toBeTruthy();
  });

  it('renders line shape', () => {
    const { getByTestId } = render(
      <AoeTemplate shape="line" originX={50} originY={50} cellSize={30} sizeInFt={30} />,
    );
    expect(getByTestId('aoe-line')).toBeTruthy();
  });

  it('renders cube shape', () => {
    const { getByTestId } = render(
      <AoeTemplate shape="cube" originX={60} originY={60} cellSize={30} sizeInFt={15} />,
    );
    expect(getByTestId('aoe-cube')).toBeTruthy();
  });
});
