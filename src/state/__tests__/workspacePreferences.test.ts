import { beforeEach, expect, it } from 'vitest';
import { useStore } from '../useStore';
import { defaultWorkspace, workspacePreferences } from '../workspacePreferences';

beforeEach(() => useStore.setState(useStore.getInitialState()));
it('migrates old reading/chat preferences and bounds malformed values', () => {
  expect(workspacePreferences(null, { chatWidth: 544, readingFontSize: 18 })).toMatchObject({
    chatWidth: 544,
    readingFontSize: 18,
    viewMode: 'split',
  });
  expect(
    workspacePreferences({
      schemaVersion: 1,
      chatWidth: 5000,
      contextWidth: -20,
      readingFontSize: 40,
      selectedContext: 'private-proof',
      viewMode: 'unknown',
    }),
  ).toMatchObject({
    chatWidth: 640,
    contextWidth: 240,
    readingFontSize: 17,
    selectedContext: null,
    viewMode: 'split',
  });
  expect(workspacePreferences({ schemaVersion: 9, chatWidth: 600 })).toEqual(defaultWorkspace());
});
it('projects only safe workspace fields and synchronizes legacy consumers on reset', () => {
  const value = workspacePreferences({
    ...defaultWorkspace(),
    secret: 'fixture-private',
    biography: 'fixture-private',
    draft: 'fixture-private',
    resume_proof: 'fixture-private',
  });
  expect(JSON.stringify(value)).not.toContain('fixture-private');
  useStore.getState().settings.setWorkspace({
    chatWidth: 544,
    readingFontSize: 18,
    selectedContext: 'journal',
    contextWidth: 448,
    viewMode: 'chat',
  });
  expect(useStore.getState().settings).toMatchObject({ chatPanelWidth: 544, readingFontSize: 18 });
  useStore.getState().settings.resetWorkspace();
  expect(useStore.getState().settings.workspace).toEqual(defaultWorkspace());
  expect(useStore.getState().settings).toMatchObject({ chatPanelWidth: 480, readingFontSize: 17 });
});
