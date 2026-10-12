import { beforeEach, expect, it } from 'vitest';
import { selectedLocalModelIsRunning } from './localMode';
import { useStore } from './useStore';

beforeEach(() => useStore.setState(useStore.getInitialState()));
it('requires a confirmed native loaded identity for the selected model', () => {
  const state = useStore.getState().localMode;
  state.selectModel('qwen3_5_4b');
  state.setRuntimeStatus({
    llm: { state: 'ready', port: 34567 },
    image: { state: 'off' },
    model_id: 'qwen3_5_4b',
  });
  expect(selectedLocalModelIsRunning(useStore.getState().localMode)).toBe(true);
  state.selectModel('qwen3_5_9b');
  state.setRuntimeStatus({
    llm: { state: 'ready', port: 34567 },
    image: { state: 'off' },
    model_id: 'qwen3_5_4b',
  });
  expect(selectedLocalModelIsRunning(useStore.getState().localMode)).toBe(false);
  state.setRuntimeStatus({
    llm: { state: 'ready', port: 34568 },
    image: { state: 'off' },
    model_id: 'qwen3_5_9b',
  });
  expect(selectedLocalModelIsRunning(useStore.getState().localMode)).toBe(true);
  state.markRuntimeStatusUnknown();
  expect(selectedLocalModelIsRunning(useStore.getState().localMode)).toBe(false);
});
it('does not infer model identity from a process port or configuration alone', () => {
  useStore
    .getState()
    .localMode.setRuntimeStatus({ llm: { state: 'ready', port: 34567 }, image: { state: 'off' } });
  expect(selectedLocalModelIsRunning(useStore.getState().localMode)).toBe(false);
});
