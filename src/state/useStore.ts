import { create } from 'zustand';
import { type ChatSlice, createChatSlice } from './chat';
import { createSettingsSlice, type SettingsSlice } from './settings';

export type AppState = ChatSlice & SettingsSlice;

export const useStore = create<AppState>()((...a) => ({
  ...createChatSlice(...a),
  ...createSettingsSlice(...a),
}));
