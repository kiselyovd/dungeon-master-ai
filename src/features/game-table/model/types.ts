import type { BattleMap } from '../../../api/contracts/battleMap';
import type { MediaReference } from '../../../api/contracts/mediaAsset';

export interface TableScope {
  mode: 'solo' | 'host' | 'guest';
  campaignId: string;
  sessionId: string;
  revision: number;
  timeline: number;
  roomId: string | null;
  roomEpoch: number | null;
  encounterId: string | null;
}
/** Public appearance/vitals only. Private sheets, proposals, credentials and
 * provider payloads never belong to the shared party projection. */
export interface TableActor {
  id: string;
  characterId: string | null;
  name: string;
  classId: string | null;
  allegiance: 'self' | 'party' | 'enemy';
  hp: number;
  maxHp: number;
  ac: number;
  life: 'healthy' | 'dying' | 'stable' | 'dead' | 'unknown';
  portraitAssetId: string | null;
  portraitMediaRef: MediaReference | null;
}
export type TableScene =
  | { kind: 'exploration'; title: string | null; art: MediaReference | null }
  | { kind: 'tactical'; title: string | null; map: BattleMap; art: MediaReference | null };
export interface TableMessage {
  id: string;
  role: 'user' | 'assistant';
  text: string;
  delivery: 'confirmed' | 'streaming';
  truncated: boolean;
}
export interface TableOutcome {
  id: string;
  key: 'outcome_dice' | 'outcome_hit_points' | 'outcome_done';
  values: Readonly<Record<string, number>>;
  failed: boolean;
}
export type TableBlockReason =
  | 'host_only'
  | 'syncing'
  | 'offline'
  | 'hero_required'
  | 'model_required'
  | 'busy'
  | 'recovery';
export type TableAvailability = { enabled: true } | { enabled: false; reason: TableBlockReason };
export interface TablePresentation {
  scope: TableScope;
  title: string;
  scene: TableScene;
  actors: readonly TableActor[];
  ownActorId: string | null;
  messages: readonly TableMessage[];
  outcomes: readonly TableOutcome[];
  connection: 'local' | 'connecting' | 'connected' | 'disconnected';
  readiness: 'ready' | 'unknown' | 'needs_model';
  recovery: 'clear' | 'checking' | 'pending' | 'unavailable';
  combat: 'inactive' | 'active' | 'ended';
  availability: {
    send: TableAvailability;
    createHero: TableAvailability;
    configure: TableAvailability;
  };
}
/** Views emit UI intentions through bound callbacks. Controllers retain request
 * identities, revision guards, transport, rules and recovery authority. */
export interface TablePresentationPorts {
  send(text: string): void | Promise<void>;
  retry(): void | Promise<void>;
  cancel(): void | Promise<void>;
  openHero(): void;
  openJournal?: () => void;
  openNpcs?: () => void;
  resume?: () => void | Promise<void>;
  configure?: () => void;
}
