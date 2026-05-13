import { useCallback, useEffect, useRef } from 'react';
import {
  type AssistField,
  streamCharacterField,
  streamFullCharacter,
  streamTestChat,
} from '../api/characterAssist';
import type { CharacterDraft, TestChatTurn } from '../state/charCreation';
import { useStore } from '../state/useStore';

type DirectFieldKey = 'name' | 'backstory' | 'ideals' | 'bonds' | 'flaws' | 'portraitPrompt';

const FIELD_MAP: Partial<Record<AssistField, DirectFieldKey>> = {
  name: 'name',
  backstory: 'backstory',
  ideals: 'ideals',
  bonds: 'bonds',
  flaws: 'flaws',
  portrait_prompt: 'portraitPrompt',
};

export interface UseCharacterAssist {
  generateField: (field: AssistField) => Promise<void>;
  surpriseMe: () => Promise<void>;
  runTestChat: (userMessage: string, history: TestChatTurn[]) => Promise<string>;
  cancel: () => void;
}

function snapshotDraft(s: ReturnType<typeof useStore.getState>): CharacterDraft {
  return {
    classId: s.charCreation.classId,
    subclassId: s.charCreation.subclassId,
    raceId: s.charCreation.raceId,
    subraceId: s.charCreation.subraceId,
    backgroundId: s.charCreation.backgroundId,
    abilityMethod: s.charCreation.abilityMethod,
    abilities: s.charCreation.abilities,
    abilityRollHistory: s.charCreation.abilityRollHistory,
    pointBuyRemaining: s.charCreation.pointBuyRemaining,
    skillProfs: s.charCreation.skillProfs,
    spells: s.charCreation.spells,
    equipmentMode: s.charCreation.equipmentMode,
    equipmentSlots: s.charCreation.equipmentSlots,
    equipmentInventory: s.charCreation.equipmentInventory,
    goldRemaining: s.charCreation.goldRemaining,
    personalityFlags: s.charCreation.personalityFlags,
    ideals: s.charCreation.ideals,
    bonds: s.charCreation.bonds,
    flaws: s.charCreation.flaws,
    backstory: s.charCreation.backstory,
    name: s.charCreation.name,
    alignment: s.charCreation.alignment,
    portraitUrl: s.charCreation.portraitUrl,
    portraitPrompt: s.charCreation.portraitPrompt,
  };
}

export function useCharacterAssist(): UseCharacterAssist {
  const setIsAssisting = useStore((s) => s.charCreation.setIsAssisting);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const applyAiSuggestion = useStore((s) => s.charCreation.applyAiSuggestion);
  const uiLanguage = useStore((s) => s.settings.uiLanguage);

  const draftRef = useRef<CharacterDraft>(snapshotDraft(useStore.getState()));

  useEffect(() => {
    return useStore.subscribe((state) => {
      draftRef.current = snapshotDraft(state);
    });
  }, []);

  const abortRef = useRef<AbortController | null>(null);

  const generateField = useCallback(
    async (field: AssistField): Promise<void> => {
      abortRef.current?.abort();
      abortRef.current = new AbortController();
      setIsAssisting(true);
      let accum = '';
      const target = FIELD_MAP[field];
      try {
        await streamCharacterField({
          field,
          draft: draftRef.current,
          locale: uiLanguage,
          signal: abortRef.current.signal,
          onToken: (text) => {
            accum += text;
            if (target) {
              setDraftField(target, accum);
            }
          },
          onError: () => {
            /* swallow - caller may surface via separate channel later */
          },
          onDone: () => {
            setIsAssisting(false);
          },
        });
      } finally {
        setIsAssisting(false);
      }
    },
    [uiLanguage, setIsAssisting, setDraftField],
  );

  const surpriseMe = useCallback(async (): Promise<void> => {
    abortRef.current?.abort();
    abortRef.current = new AbortController();
    setIsAssisting(true);
    try {
      await streamFullCharacter({
        draft: draftRef.current,
        locale: uiLanguage,
        signal: abortRef.current.signal,
        onPatch: (patch) => {
          applyAiSuggestion(patch);
        },
        onError: () => {},
        onDone: () => {
          setIsAssisting(false);
        },
      });
    } finally {
      setIsAssisting(false);
    }
  }, [uiLanguage, setIsAssisting, applyAiSuggestion]);

  const runTestChat = useCallback(
    async (userMessage: string, history: TestChatTurn[]): Promise<string> => {
      abortRef.current?.abort();
      abortRef.current = new AbortController();
      setIsAssisting(true);
      let reply = '';
      try {
        await streamTestChat({
          draft: draftRef.current,
          history,
          userMessage,
          locale: uiLanguage,
          signal: abortRef.current.signal,
          onToken: (t) => {
            reply += t;
          },
          onError: () => {},
          onDone: () => {
            setIsAssisting(false);
          },
        });
      } finally {
        setIsAssisting(false);
      }
      return reply;
    },
    [uiLanguage, setIsAssisting],
  );

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    setIsAssisting(false);
  }, [setIsAssisting]);

  return { generateField, surpriseMe, runTestChat, cancel };
}
