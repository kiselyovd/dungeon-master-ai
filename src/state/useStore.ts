import { create } from 'zustand';
import { createChatSlice, type ChatSlice } from './chat';
import { createSettingsSlice, type SettingsSlice } from './settings';

export type AppState = ChatSlice & SettingsSlice;

export const useStore = create<AppState>()((...a) => ({
  ...createChatSlice(...a),
  ...createSettingsSlice(...a),
}));
