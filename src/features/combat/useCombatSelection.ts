import { useEffect, useState } from 'react';
import type { CombatToken } from './model/types';

/** Selection is a local reading/target hint, never permission to command an actor. */
export function useCombatSelection(scope: string, tokens: readonly CombatToken[]) {
  const [choice, setChoice] = useState<{ scope: string; id: string } | null>(null);
  const selectedId =
    choice?.scope === scope && tokens.some((token) => token.id === choice.id) ? choice.id : null;
  useEffect(() => {
    setChoice((current) => (current?.scope === scope ? current : null));
  }, [scope]);
  return {
    selectedId,
    select: (id: string) => {
      if (tokens.some((token) => token.id === id)) setChoice({ scope, id });
    },
  };
}
