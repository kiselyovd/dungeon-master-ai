import heroCleric from './char-portrait-cleric.png';
import heroFighter from './char-portrait-fighter.png';
import heroRogue from './char-portrait-rogue.png';
import heroWizard from './char-portrait-wizard.png';
import npcGuard from './npc-fallback-guard.png';
import npcInnkeeper from './npc-fallback-innkeeper.png';
import npcKnight from './npc-fallback-knight.png';
import npcMage from './npc-fallback-mage.png';
import npcMerchant from './npc-fallback-merchant.png';
import npcPeasant from './npc-fallback-peasant.png';
import npcPriestess from './npc-fallback-priestess.png';
import npcRogue from './npc-fallback-rogue.png';
import onboarding from './onboarding-hero.png';
import saveCombat from './save-thumb-combat.png';
import saveDialog from './save-thumb-dialog.png';
import saveExploration from './save-thumb-exploration.png';
import saveNpc from './save-thumb-npc.png';
import sceneCombat from './scene-transition-combat.mp4';
import sceneDialog from './scene-transition-dialog.mp4';
import sceneDungeon from './scene-transition-dungeon.mp4';
import sceneExploration from './scene-transition-exploration.mp4';
import splash from './splash.png';
import tokenCleric from './token-cleric.png';
import tokenFighter from './token-fighter.png';
import tokenRogue from './token-rogue.png';
import tokenWizard from './token-wizard.png';
import vttEmpty from './vtt-empty.png';

export const KEY_ART = { splash, onboarding, vttEmpty } as const;

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
