/**
 * PersistStorage adapter for Zustand's persist middleware. Splits the
 * persisted Settings slice across two on-disk surfaces:
 *
 * - Secrets (provider configs + Replicate API key) live in an encrypted
 *   Stronghold vault via `strongholdSecretsStore`. The on-disk file
 *   `dmai-vault.hold` is argon2-keyed with a per-install salt.
 * - Non-sensitive prefs (active provider, languages, system prompt,
 *   temperature, active campaign / session ID) live in `settings.json`
 *   via plaintext `LazyStore` (tauri-plugin-store).
 *
 * The split intentionally keeps the secrets surface narrow so the
 * Stronghold backend only handles credential-bearing fields. Migration
 * from the pre-M5 plaintext `secrets.json` is one-shot on first read:
 * if a value is found in the legacy file but not yet in Stronghold, we
 * copy it over and delete the legacy entry.
 *
 * The settings file's key layout still matches the M1.5 `loadAll/save*`
 * helpers (`providers`, `active_provider`, `ui_language`, `narration_language`)
 * so an upgrade from M1.5 keeps the saved prefs.
 */
import { LazyStore } from '@tauri-apps/plugin-store';
import * as v from 'valibot';
import type { PersistStorage, StorageValue } from 'zustand/middleware';
import type { CharacterDraft, WizardTab } from './charCreation';
import { CharCreationDraftSchema } from './charCreationSchema';
import type { OnboardingData } from './onboarding';
import type { PcData } from './pc';
import {
  AnthropicConfigSchema,
  LocalMistralRsConfigSchema,
  OpenaiCompatConfigSchema,
} from './providers';
import type { CurrentScene, SessionData } from './session';
import {
  type Language,
  MAX_CHAT_WIDTH,
  MIN_CHAT_WIDTH,
  type ProvidersMap,
  type SettingsData,
} from './settings';
import { strongholdSecretsStore } from './strongholdSecretsStore';

const LEGACY_SECRETS_FILE = 'secrets.json';
const SETTINGS_FILE = 'settings.json';

const KEY_PROVIDERS = 'providers';
const KEY_ACTIVE_PROVIDER = 'active_provider';
const KEY_UI_LANGUAGE = 'ui_language';
const KEY_NARRATION_LANGUAGE = 'narration_language';
const KEY_SYSTEM_PROMPT = 'system_prompt';
const KEY_TEMPERATURE = 'temperature';
const KEY_REPLICATE_API_KEY = 'replicate_api_key';
const KEY_CHAT_PANEL_WIDTH = 'chat_panel_width';
const KEY_SCENE_TRANSITIONS_ENABLED = 'scene_transitions_enabled';
const KEY_ACTIVE_CAMPAIGN_ID = 'active_campaign_id';
const KEY_ACTIVE_SESSION_ID = 'active_session_id';
const KEY_CURRENT_SCENE = 'current_scene';
const KEY_ONBOARDING_COMPLETED = 'onboarding_completed';
const KEY_HERO_CLASS = 'hero_class';
const KEY_PC = 'pc';
const KEY_CHAR_CREATION_DRAFT = 'char_creation_draft';

const secretsStore = strongholdSecretsStore;
const legacySecretsStore = new LazyStore(LEGACY_SECRETS_FILE);
const settingsStore = new LazyStore(SETTINGS_FILE);

/**
 * Pull a secret with one-shot migration from the pre-M5 plaintext
 * secrets.json. Returns `undefined` if neither store has the key.
 */
async function getSecret(key: string): Promise<unknown> {
  const fromVault = await secretsStore.get(key);
  if (fromVault !== undefined) return fromVault;
  const fromLegacy = await legacySecretsStore.get(key);
  if (fromLegacy === undefined || fromLegacy === null) return undefined;
  await secretsStore.set(key, fromLegacy);
  await legacySecretsStore.delete(key);
  await Promise.all([secretsStore.save(), legacySecretsStore.save()]);
  return fromLegacy;
}

const ProviderKindSchema = v.picklist(['anthropic', 'openai-compat', 'local-mistralrs']);
const LanguageSchema = v.picklist(['en', 'ru']);

const ProvidersMapSchema = v.object({
  anthropic: v.nullable(AnthropicConfigSchema),
  'openai-compat': v.nullable(OpenaiCompatConfigSchema),
  'local-mistralrs': v.nullable(LocalMistralRsConfigSchema),
});

const SystemPromptSchema = v.string();
const TemperatureSchema = v.pipe(v.number(), v.minValue(0), v.maxValue(2));
const ReplicateKeySchema = v.nullable(v.string());
const ChatPanelWidthSchema = v.pipe(
  v.number(),
  v.minValue(MIN_CHAT_WIDTH),
  v.maxValue(MAX_CHAT_WIDTH),
);
const SceneTransitionsEnabledSchema = v.boolean();
const SessionIdSchema = v.string();
const CurrentSceneSchema = v.nullable(
  v.object({
    name: v.string(),
    stepCounter: v.pipe(v.number(), v.integer(), v.minValue(0)),
  }),
);
const OnboardingCompletedSchema = v.boolean();
const HeroClassSchema = v.nullable(v.string());

const AbilityScoresSchema = v.object({
  str: v.number(),
  dex: v.number(),
  con: v.number(),
  int: v.number(),
  wis: v.number(),
  cha: v.number(),
});

const SavingThrowProfSchema = v.object({
  str: v.optional(v.boolean()),
  dex: v.optional(v.boolean()),
  con: v.optional(v.boolean()),
  int: v.optional(v.boolean()),
  wis: v.optional(v.boolean()),
  cha: v.optional(v.boolean()),
});

const SkillProfSchema = v.object({
  acrobatics: v.optional(v.boolean()),
  athletics: v.optional(v.boolean()),
  arcana: v.optional(v.boolean()),
  deception: v.optional(v.boolean()),
  history: v.optional(v.boolean()),
  insight: v.optional(v.boolean()),
  intimidation: v.optional(v.boolean()),
  investigation: v.optional(v.boolean()),
  perception: v.optional(v.boolean()),
  persuasion: v.optional(v.boolean()),
  stealth: v.optional(v.boolean()),
  survival: v.optional(v.boolean()),
});

const InventoryItemSchema = v.object({
  id: v.string(),
  name: v.string(),
  count: v.number(),
  icon: v.optional(v.string()),
});

const PcSchema = v.object({
  heroClass: v.nullable(v.string()),
  name: v.nullable(v.string()),
  race: v.nullable(v.string()),
  subclass: v.nullable(v.string()),
  background: v.nullable(v.string()),
  alignment: v.nullable(v.string()),
  level: v.number(),
  experience: v.number(),
  experienceNext: v.number(),
  hp: v.number(),
  hpMax: v.number(),
  ac: v.number(),
  initiative: v.number(),
  speedFt: v.number(),
  proficiencyBonus: v.number(),
  abilities: AbilityScoresSchema,
  savingThrowProfs: SavingThrowProfSchema,
  skillProfs: SkillProfSchema,
  inventory: v.array(InventoryItemSchema),
});

export interface PersistedSettings {
  settings?: Partial<SettingsData>;
  session?: Partial<SessionData>;
  onboarding?: Partial<OnboardingData>;
  pc?: Partial<PcData>;
  charCreation?: Partial<CharacterDraft & { activeTab: WizardTab }>;
}

export const persistStorage: PersistStorage<PersistedSettings> = {
  async getItem(_name): Promise<StorageValue<PersistedSettings> | null> {
    const [
      providersRaw,
      activeRaw,
      uiRaw,
      narrRaw,
      sysRaw,
      tempRaw,
      replicateRaw,
      chatWidthRaw,
      sceneTransitionsRaw,
      campaignRaw,
      sessionIdRaw,
      sceneRaw,
      onboardingRaw,
      heroClassRaw,
      pcRaw,
      charCreationRaw,
    ] = await Promise.all([
      getSecret(KEY_PROVIDERS),
      settingsStore.get(KEY_ACTIVE_PROVIDER),
      settingsStore.get(KEY_UI_LANGUAGE),
      settingsStore.get(KEY_NARRATION_LANGUAGE),
      settingsStore.get(KEY_SYSTEM_PROMPT),
      settingsStore.get(KEY_TEMPERATURE),
      getSecret(KEY_REPLICATE_API_KEY),
      settingsStore.get(KEY_CHAT_PANEL_WIDTH),
      settingsStore.get(KEY_SCENE_TRANSITIONS_ENABLED),
      settingsStore.get(KEY_ACTIVE_CAMPAIGN_ID),
      settingsStore.get(KEY_ACTIVE_SESSION_ID),
      settingsStore.get(KEY_CURRENT_SCENE),
      settingsStore.get(KEY_ONBOARDING_COMPLETED),
      settingsStore.get(KEY_HERO_CLASS),
      settingsStore.get(KEY_PC),
      settingsStore.get(KEY_CHAR_CREATION_DRAFT),
    ]);

    const providersParsed = v.safeParse(ProvidersMapSchema, providersRaw);
    const activeParsed = v.safeParse(ProviderKindSchema, activeRaw);
    const uiParsed = v.safeParse(LanguageSchema, uiRaw);
    const narrParsed = v.safeParse(LanguageSchema, narrRaw);
    const sysParsed = v.safeParse(SystemPromptSchema, sysRaw);
    const tempParsed = v.safeParse(TemperatureSchema, tempRaw);
    const replicateParsed = v.safeParse(ReplicateKeySchema, replicateRaw);
    const chatWidthParsed = v.safeParse(ChatPanelWidthSchema, chatWidthRaw);
    const sceneTransitionsParsed = v.safeParse(SceneTransitionsEnabledSchema, sceneTransitionsRaw);
    const campaignParsed = v.safeParse(SessionIdSchema, campaignRaw);
    const sessionIdParsed = v.safeParse(SessionIdSchema, sessionIdRaw);
    const sceneParsed = v.safeParse(CurrentSceneSchema, sceneRaw);
    const onboardingParsed = v.safeParse(OnboardingCompletedSchema, onboardingRaw);
    const heroClassParsed = v.safeParse(HeroClassSchema, heroClassRaw);
    const pcParsed = v.safeParse(PcSchema, pcRaw);
    const charCreationParsed = v.safeParse(CharCreationDraftSchema, charCreationRaw);

    if (
      !providersParsed.success &&
      !activeParsed.success &&
      !uiParsed.success &&
      !narrParsed.success &&
      !sysParsed.success &&
      !tempParsed.success &&
      !replicateParsed.success &&
      !chatWidthParsed.success &&
      !sceneTransitionsParsed.success &&
      !campaignParsed.success &&
      !sessionIdParsed.success &&
      !sceneParsed.success &&
      !onboardingParsed.success &&
      !heroClassParsed.success &&
      !pcParsed.success &&
      !charCreationParsed.success
    ) {
      return null;
    }

    const settings: Partial<SettingsData> = {};
    if (providersParsed.success) settings.providers = providersParsed.output as ProvidersMap;
    if (activeParsed.success) settings.activeProvider = activeParsed.output;
    if (uiParsed.success) settings.uiLanguage = uiParsed.output as Language;
    if (narrParsed.success) settings.narrationLanguage = narrParsed.output as Language;
    if (sysParsed.success) settings.systemPrompt = sysParsed.output;
    if (tempParsed.success) settings.temperature = tempParsed.output;
    if (replicateParsed.success) settings.replicateApiKey = replicateParsed.output;
    if (chatWidthParsed.success) settings.chatPanelWidth = chatWidthParsed.output;
    if (sceneTransitionsParsed.success) {
      settings.sceneTransitionsEnabled = sceneTransitionsParsed.output;
    }

    const session: Partial<SessionData> = {};
    if (campaignParsed.success) session.activeCampaignId = campaignParsed.output;
    if (sessionIdParsed.success) session.activeSessionId = sessionIdParsed.output;
    if (sceneParsed.success) session.currentScene = sceneParsed.output as CurrentScene | null;

    const onboarding: Partial<OnboardingData> = {};
    if (onboardingParsed.success) onboarding.completed = onboardingParsed.output;

    // PC: prefer the full `pc` JSON entry; fall back to the legacy
    // `hero_class` scalar so an upgrade from M5 P2.12 (where only the
    // class was persisted) still rehydrates that field.
    let pc: Partial<PcData> = {};
    if (pcParsed.success) {
      pc = pcParsed.output as PcData;
    } else if (heroClassParsed.success) {
      pc.heroClass = heroClassParsed.output;
    }

    const stateOut: PersistedSettings = { settings, session, onboarding, pc };
    if (charCreationParsed.success) {
      stateOut.charCreation = charCreationParsed.output as Partial<
        CharacterDraft & { activeTab: WizardTab }
      >;
    }

    return { state: stateOut, version: 0 };
  },

  async setItem(_name, value): Promise<void> {
    const settings = value.state.settings ?? {};
    const session = value.state.session ?? {};
    const onboarding = value.state.onboarding ?? {};
    const pc = value.state.pc ?? {};
    const charCreation = value.state.charCreation;
    const writes: Promise<unknown>[] = [];
    if (settings.providers !== undefined) {
      writes.push(secretsStore.set(KEY_PROVIDERS, settings.providers));
    }
    if (settings.activeProvider !== undefined) {
      writes.push(settingsStore.set(KEY_ACTIVE_PROVIDER, settings.activeProvider));
    }
    if (settings.uiLanguage !== undefined) {
      writes.push(settingsStore.set(KEY_UI_LANGUAGE, settings.uiLanguage));
    }
    if (settings.narrationLanguage !== undefined) {
      writes.push(settingsStore.set(KEY_NARRATION_LANGUAGE, settings.narrationLanguage));
    }
    if (settings.systemPrompt !== undefined) {
      writes.push(settingsStore.set(KEY_SYSTEM_PROMPT, settings.systemPrompt));
    }
    if (settings.temperature !== undefined) {
      writes.push(settingsStore.set(KEY_TEMPERATURE, settings.temperature));
    }
    if (settings.replicateApiKey !== undefined) {
      writes.push(secretsStore.set(KEY_REPLICATE_API_KEY, settings.replicateApiKey));
    }
    if (settings.chatPanelWidth !== undefined) {
      writes.push(settingsStore.set(KEY_CHAT_PANEL_WIDTH, settings.chatPanelWidth));
    }
    if (settings.sceneTransitionsEnabled !== undefined) {
      writes.push(
        settingsStore.set(KEY_SCENE_TRANSITIONS_ENABLED, settings.sceneTransitionsEnabled),
      );
    }
    if (session.activeCampaignId !== undefined && session.activeCampaignId !== null) {
      writes.push(settingsStore.set(KEY_ACTIVE_CAMPAIGN_ID, session.activeCampaignId));
    }
    if (session.activeSessionId !== undefined && session.activeSessionId !== null) {
      writes.push(settingsStore.set(KEY_ACTIVE_SESSION_ID, session.activeSessionId));
    }
    if (session.currentScene !== undefined) {
      writes.push(settingsStore.set(KEY_CURRENT_SCENE, session.currentScene));
    }
    if (onboarding.completed !== undefined) {
      writes.push(settingsStore.set(KEY_ONBOARDING_COMPLETED, onboarding.completed));
    }
    if (pc.heroClass !== undefined) {
      // Keep the legacy scalar in sync so a downgrade still finds the class.
      writes.push(settingsStore.set(KEY_HERO_CLASS, pc.heroClass));
    }
    // Persist the full PC JSON whenever any of its fields changed. The
    // partialize layer in useStore.ts always sends the whole `pc` slice,
    // so checking `name` (a sentinel for "character has been created")
    // is enough to know whether to write the JSON entry.
    if (pc.name !== undefined || pc.heroClass !== undefined) {
      writes.push(settingsStore.set(KEY_PC, pc));
    }
    if (charCreation !== undefined && Object.keys(charCreation).length > 0) {
      writes.push(settingsStore.set(KEY_CHAR_CREATION_DRAFT, charCreation));
    }
    await Promise.all(writes);
    await Promise.all([secretsStore.save(), settingsStore.save()]);
  },

  async removeItem(_name): Promise<void> {
    await Promise.all([
      secretsStore.delete(KEY_PROVIDERS),
      secretsStore.delete(KEY_REPLICATE_API_KEY),
      legacySecretsStore.delete(KEY_PROVIDERS),
      legacySecretsStore.delete(KEY_REPLICATE_API_KEY),
      settingsStore.delete(KEY_ACTIVE_PROVIDER),
      settingsStore.delete(KEY_UI_LANGUAGE),
      settingsStore.delete(KEY_NARRATION_LANGUAGE),
      settingsStore.delete(KEY_SYSTEM_PROMPT),
      settingsStore.delete(KEY_TEMPERATURE),
      settingsStore.delete(KEY_CHAT_PANEL_WIDTH),
      settingsStore.delete(KEY_SCENE_TRANSITIONS_ENABLED),
      settingsStore.delete(KEY_ACTIVE_CAMPAIGN_ID),
      settingsStore.delete(KEY_ACTIVE_SESSION_ID),
      settingsStore.delete(KEY_CURRENT_SCENE),
      settingsStore.delete(KEY_ONBOARDING_COMPLETED),
      settingsStore.delete(KEY_HERO_CLASS),
      settingsStore.delete(KEY_PC),
      settingsStore.delete(KEY_CHAR_CREATION_DRAFT),
    ]);
    await Promise.all([secretsStore.save(), legacySecretsStore.save(), settingsStore.save()]);
  },
};
