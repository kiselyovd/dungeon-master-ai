import heroCleric from './living-tabletop/hero-cleric.webp';
import heroFighter from './living-tabletop/hero-fighter.webp';
import heroRogue from './living-tabletop/hero-rogue.webp';
import heroWizard from './living-tabletop/hero-wizard.webp';
import npcGuard from './living-tabletop/npc-guard.webp';
import npcInnkeeper from './living-tabletop/npc-innkeeper.webp';
import npcKnight from './living-tabletop/npc-knight.webp';
import npcMage from './living-tabletop/npc-mage.webp';
import npcMerchant from './living-tabletop/npc-merchant.webp';
import npcPeasant from './living-tabletop/npc-peasant.webp';
import npcPriestess from './living-tabletop/npc-priestess.webp';
import npcRogue from './living-tabletop/npc-rogue.webp';
import sceneCombat from './living-tabletop/scene-combat.webp';
import sceneDialog from './living-tabletop/scene-dialog.webp';
import sceneDungeon from './living-tabletop/scene-dungeon.webp';
import sceneExploration from './living-tabletop/scene-exploration.webp';
import tokenCleric from './living-tabletop/token-cleric.png';
import tokenFighter from './living-tabletop/token-fighter.png';
import tokenRogue from './living-tabletop/token-rogue.png';
import tokenWizard from './living-tabletop/token-wizard.png';
import vttEmpty from './living-tabletop/v2/key-exploration-v2.webp';
import onboarding from './living-tabletop/v2/key-onboarding-v2.webp';
import splash from './living-tabletop/v2/key-splash-v2.webp';
import saveCombat from './living-tabletop/v2/save-combat-v2.webp';
import saveDialog from './living-tabletop/v2/save-dialog-v2.webp';
import saveExploration from './living-tabletop/v2/save-exploration-v2.webp';
import saveNpc from './living-tabletop/v2/save-npc-v2.webp';

export const KEY_ART = { splash, onboarding, vttEmpty } as const;
export const KEY_ART_IDS = {
  splash: 'splash.v2',
  onboarding: 'onboarding.v2',
  vttEmpty: 'vtt.empty.v2',
} as const;
export const SAVE_ART_IDS = {
  combat: 'save.combat.v2',
  exploration: 'save.exploration.v2',
  dialog: 'save.dialog.v2',
  npc: 'save.npc.v2',
} as const;

export const HERO_ART = {
  fighter: heroFighter,
  wizard: heroWizard,
  rogue: heroRogue,
  cleric: heroCleric,
} as const;

export const TOKEN_ART = {
  fighter: tokenFighter,
  wizard: tokenWizard,
  rogue: tokenRogue,
  cleric: tokenCleric,
} as const;

export const NPC_ART = {
  innkeeper: npcInnkeeper,
  guard: npcGuard,
  merchant: npcMerchant,
  rogue: npcRogue,
  mage: npcMage,
  priestess: npcPriestess,
  knight: npcKnight,
  peasant: npcPeasant,
} as const;

export const SAVE_ART = {
  combat: saveCombat,
  exploration: saveExploration,
  dialog: saveDialog,
  npc: saveNpc,
} as const;

export const SCENE_ART = {
  combat: sceneCombat,
  dialog: sceneDialog,
  exploration: sceneExploration,
  dungeon: sceneDungeon,
} as const;
