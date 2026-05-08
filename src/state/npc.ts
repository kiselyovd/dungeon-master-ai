import type { StateCreator } from 'zustand';

export interface NpcFact {
  text: string;
  created_at: string;
}

export const DISPOSITIONS = ['friendly', 'neutral', 'hostile', 'unknown'] as const;
export type Disposition = (typeof DISPOSITIONS)[number];

export interface NpcRecord {
  id: string;
  campaign_id: string;
  name: string;
  role: string;
  disposition: Disposition;
  trust: number;
  facts: NpcFact[];
  updated_at: string;
}

export interface NpcSlice {
  npcs: {
    records: Record<string, NpcRecord>; // keyed by name
    isOpen: boolean;
    upsertNpc: (npc: NpcRecord) => void;
    addFact: (name: string, fact: NpcFact) => void;
    setNpcs: (npcs: NpcRecord[]) => void;
    open: () => void;
    close: () => void;
  };
}

export const createNpcSlice: StateCreator<NpcSlice, [], [], NpcSlice> = (set) => ({
  npcs: {
    records: {},
    isOpen: false,

    upsertNpc: (npc) =>
      set((s) => ({ npcs: { ...s.npcs, records: { ...s.npcs.records, [npc.name]: npc } } })),

    addFact: (name, fact) =>
      set((s) => {
        const existing = s.npcs.records[name];
        if (!existing) return s;
        return {
          npcs: {
            ...s.npcs,
            records: {
              ...s.npcs.records,
              [name]: { ...existing, facts: [...existing.facts, fact] },
            },
          },
        };
      }),

    setNpcs: (npcs) =>
      set((s) => ({
        npcs: {
          ...s.npcs,
          records: Object.fromEntries(npcs.map((n) => [n.name, n])),
        },
      })),

    open: () => set((s) => ({ npcs: { ...s.npcs, isOpen: true } })),
    close: () => set((s) => ({ npcs: { ...s.npcs, isOpen: false } })),
  },
});
