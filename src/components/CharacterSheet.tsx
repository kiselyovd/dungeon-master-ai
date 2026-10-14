import type { ReactNode } from 'react';
import { useMediaImage } from '../features/media/useMediaImage';
import { useStore } from '../state/useStore';
import { CharacterSheetView } from './CharacterSheetView';

interface CharacterSheetProps {
  tools?: ReactNode;
  open: boolean;
  onClose: () => void;
  onRest?: (() => void) | undefined;
}
export function CharacterSheet({ open, onClose, onRest, tools }: CharacterSheetProps) {
  const pc = useStore((state) => state.pc);
  const campaignId = useStore((state) => state.session.activeCampaignId);
  const setup = useStore((state) => state.onboarding.reset);
  const media = useMediaImage(
    open ? pc.portraitMediaRef : null,
    campaignId ? { kind: 'campaign', campaign_id: campaignId } : null,
    'portrait',
  );
  return (
    <CharacterSheetView
      open={open}
      pc={pc}
      portraitUrl={media.url}
      onClose={onClose}
      onSetup={setup}
      onRest={onRest}
      tools={tools}
    />
  );
}
