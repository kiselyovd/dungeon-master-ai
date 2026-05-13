import type { Compendium } from '../../api/srd';
import type { CharacterWizardMode } from '../CharacterWizard';

export function ReviewTab(_props: {
  compendium: Compendium;
  mode: CharacterWizardMode;
  onClose?: () => void;
}) {
  return <div>review tab placeholder</div>;
}
