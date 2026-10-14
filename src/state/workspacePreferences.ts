export const MIN_CHAT_WIDTH = 360;
export const MAX_CHAT_WIDTH = 640;
export const DEFAULT_CHAT_WIDTH = 480;
export const READING_FONT_SIZES = [16, 17, 18] as const;
export type ReadingFontSize = (typeof READING_FONT_SIZES)[number];
export const DEFAULT_READING_FONT_SIZE: ReadingFontSize = 17;
export const MIN_CONTEXT_WIDTH = 240;
export const MAX_CONTEXT_WIDTH = 520;
export const DEFAULT_CONTEXT_WIDTH = 380;
export const CONTEXT_PANELS = ['adventure', 'hero', 'journal', 'npcs', 'context'] as const;
export type ContextPanel = (typeof CONTEXT_PANELS)[number];
export type WorkspaceView = 'split' | 'scene' | 'chat';
export interface WorkspacePreferences {
  schemaVersion: 1;
  chatWidth: number;
  contextWidth: number;
  readingFontSize: ReadingFontSize;
  selectedContext: ContextPanel | null;
  expandedContext: boolean;
  viewMode: WorkspaceView;
}
export function defaultWorkspace(): WorkspacePreferences {
  return {
    schemaVersion: 1,
    chatWidth: DEFAULT_CHAT_WIDTH,
    contextWidth: DEFAULT_CONTEXT_WIDTH,
    readingFontSize: DEFAULT_READING_FONT_SIZE,
    selectedContext: null,
    expandedContext: false,
    viewMode: 'split',
  };
}
function finiteWidth(value: unknown, min: number, max: number, fallback: number) {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.round(Math.min(max, Math.max(min, value)))
    : fallback;
}
/** Explicit projection strips unknown fields, including any private payloads. */
export function workspacePreferences(
  value: unknown,
  legacy: { chatWidth?: unknown; readingFontSize?: unknown } = {},
): WorkspacePreferences {
  const defaults = defaultWorkspace();
  const record =
    value && typeof value === 'object' && 'schemaVersion' in value && value.schemaVersion === 1
      ? (value as Record<string, unknown>)
      : {};
  const reading = record.readingFontSize ?? legacy.readingFontSize;
  return {
    schemaVersion: 1,
    chatWidth: finiteWidth(
      record.chatWidth ?? legacy.chatWidth,
      MIN_CHAT_WIDTH,
      MAX_CHAT_WIDTH,
      defaults.chatWidth,
    ),
    contextWidth: finiteWidth(
      record.contextWidth,
      MIN_CONTEXT_WIDTH,
      MAX_CONTEXT_WIDTH,
      defaults.contextWidth,
    ),
    readingFontSize:
      READING_FONT_SIZES.find((size) => size === reading) ?? defaults.readingFontSize,
    selectedContext: CONTEXT_PANELS.find((panel) => panel === record.selectedContext) ?? null,
    expandedContext: record.expandedContext === true,
    viewMode: record.viewMode === 'scene' || record.viewMode === 'chat' ? record.viewMode : 'split',
  };
}
