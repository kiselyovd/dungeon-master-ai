import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Compendium } from '../../api/srd';
import { useCharacterAssist } from '../../hooks/useCharacterAssist';
import type { TestChatTurn } from '../../state/charCreation';
import { emptyPcData } from '../../state/pc';
import { useStore } from '../../state/useStore';
import type { CharacterWizardMode } from '../CharacterWizard';
import { DmConfirmModal } from '../DmConfirmModal';
import { computeLiveSheet } from './computeLiveSheet';
import { parseEquipmentString, resolveDraftInventory } from './equipmentResolver';

interface Warning {
  code: string;
  severity: 'block' | 'warn';
}

export interface ReviewTabProps {
  completeSoloOnboarding?: boolean;
  onCommitCharacter?: (character: Record<string, unknown>) => Promise<void>;
  compendium: Compendium;
  mode: CharacterWizardMode;
  onClose?: () => void;
}

export function ReviewTab({
  compendium,
  mode,
  onClose,
  onCommitCharacter,
  completeSoloOnboarding = true,
}: ReviewTabProps) {
  const { t } = useTranslation('wizard');
  const draft = useStore((s) => s.charCreation);
  const resetDraft = useStore((s) => s.charCreation.resetDraft);
  const completeOnboarding = useStore((s) => s.onboarding.complete);
  const { runTestChat, cancel } = useCharacterAssist();

  const [history, setHistory] = useState<TestChatTurn[]>([]);
  const [userInput, setUserInput] = useState('');
  const [chatBusy, setChatBusy] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [committing, setCommitting] = useState(false);
  const [commitError, setCommitError] = useState<string | null>(null);

  const warnings: Warning[] = [];
  if (!draft.classId) warnings.push({ code: 'no_class', severity: 'block' });
  if (!draft.raceId) warnings.push({ code: 'no_race', severity: 'block' });
  if (!draft.backgroundId) warnings.push({ code: 'no_background', severity: 'block' });
  if (!draft.abilityMethod) warnings.push({ code: 'no_ability_method', severity: 'block' });
  if (!draft.name) warnings.push({ code: 'no_name', severity: 'warn' });
  if (draft.equipmentMode === null) warnings.push({ code: 'no_equipment', severity: 'warn' });

  if (draft.equipmentMode === 'package') {
    const unresolved = draft.equipmentSlots.some((slot) => {
      if (slot.resolvedItemIds.length > 0) return false;
      const chunks = (slot.customName ?? '')
        .split(',')
        .map((c) => c.trim())
        .filter(Boolean);
      return chunks.some((c) => parseEquipmentString(c).isWildcard);
    });
    if (unresolved) warnings.push({ code: 'unresolved_wildcard', severity: 'warn' });
  }

  if (computeLiveSheet(draft, compendium).defense?.valid === false)
    warnings.push({ code: 'defense_invalid', severity: 'block' });

  const blocking = warnings.some((w) => w.severity === 'block');

  async function sendTestChat() {
    if (!userInput.trim()) return;
    setChatBusy(true);
    const msg = userInput;
    setUserInput('');
    const newHistory: TestChatTurn[] = [...history, { role: 'pc', text: msg }];
    setHistory(newHistory);
    const reply = await runTestChat(msg, newHistory);
    setHistory([...newHistory, { role: 'npc', text: reply }]);
    setChatBusy(false);
  }

  function begin() {
    if (mode === 'edit') {
      setConfirmOpen(true);
      return;
    }
    void beginConfirmed();
  }

  async function beginConfirmed() {
    if (committing || blocking) return;
    setConfirmOpen(false);
    const inventory = resolveDraftInventory(draft, compendium);

    // Derive the real combat stats from the same live-sheet math the preview
    // uses, so a wizard-built character commits non-zero HP and a real AC
    // instead of the EMPTY_PC defaults (hp 0/0, ac 10, initiative 0).
    const sheet = computeLiveSheet(draft, compendium);
    const derivedAbilities = {
      str: sheet.abilities.str.score,
      dex: sheet.abilities.dex.score,
      con: sheet.abilities.con.score,
      int: sheet.abilities.int.score,
      wis: sheet.abilities.wis.score,
      cha: sheet.abilities.cha.score,
    };

    const character = {
      ...emptyPcData(),
      heroClass: draft.classId,
      name: draft.name.trim() || t('placeholder_hero'),
      race: draft.raceId,
      subclass: draft.subclassId,
      background: draft.backgroundId,
      alignment: draft.alignment,
      level: 1,
      abilities: derivedAbilities,
      hp: sheet.hp ?? 0,
      hpMax: sheet.hpMax ?? 0,
      ac: sheet.ac ?? 10,
      ...(sheet.defense ? { defense: sheet.defense.choice } : {}),
      baseSpeedFt: sheet.baseSpeedFt ?? 30,
      initiative: sheet.initiative ?? 0,
      speedFt: sheet.speedFt ?? 30,
      proficiencyBonus: sheet.proficiencyBonus,
      portraitUrl: draft.portraitUrl,
      inventory,
      savingThrowProfs: Object.fromEntries(
        Object.entries(sheet.savingThrows).map(([key, value]) => [key, value.proficient]),
      ),
      skillProfs: Object.fromEntries(
        Object.entries(sheet.skills).map(([key, value]) => [key, value.proficient]),
      ),
      backstory: draft.backstory,
      ideals: draft.ideals,
      bonds: draft.bonds,
      flaws: draft.flaws,
      personalityFlags: draft.personalityFlags,
      spells: draft.spells,
    };
    if ((character.portraitUrl?.length ?? 0) > 8 * 1024 * 1024) {
      setCommitError('portrait_too_large');
      return;
    }
    setCommitting(true);
    setCommitError(null);
    try {
      if (!onCommitCharacter) throw new Error('campaign_required');
      await onCommitCharacter(character);
      resetDraft();
      if (completeSoloOnboarding) completeOnboarding();
      onClose?.();
    } catch {
      setCommitError('character_commit_failed');
    } finally {
      setCommitting(false);
    }
  }

  return (
    <section>
      <h2>{t('review_title')}</h2>

      <div className="dm-wizard-card" style={{ cursor: 'default', marginBottom: 16 }}>
        <h3 style={{ fontFamily: 'Cinzel, serif', color: '#fff' }}>
          {draft.name || t('placeholder_hero')}
        </h3>
        <p style={{ color: 'var(--color-text-muted)', fontSize: 12 }}>
          {[draft.classId, draft.raceId, draft.backgroundId].filter(Boolean).join(' - ') ||
            t('summary_empty')}
        </p>
      </div>

      <section style={{ marginTop: 24 }}>
        <h3>{t('test_chat_title')}</h3>
        {history.length === 0 && (
          <p style={{ color: 'var(--color-text-muted)', fontSize: 12 }}>{t('test_chat_intro')}</p>
        )}
        <ul style={{ listStyle: 'none', padding: 0 }}>
          {history.map((turn, idx) => (
            <li
              key={idx}
              style={{
                padding: 6,
                marginBottom: 4,
                background:
                  turn.role === 'npc' ? 'rgba(212,175,55,0.08)' : 'rgba(255,255,255,0.04)',
                borderRadius: 4,
              }}
            >
              <span
                style={{
                  fontSize: 10,
                  color: 'var(--color-accent)',
                  marginRight: 6,
                  letterSpacing: '0.1em',
                }}
              >
                {turn.role === 'npc' ? 'NPC' : t('you')}:
              </span>
              {turn.text}
            </li>
          ))}
        </ul>
        <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
          <input
            value={userInput}
            onChange={(e) => setUserInput(e.target.value)}
            placeholder={t('test_chat_placeholder')}
            style={{ flex: 1, padding: 8 }}
            disabled={chatBusy}
          />
          <button
            type="button"
            className="dm-wizard-btn-secondary"
            onClick={chatBusy ? cancel : () => void sendTestChat()}
          >
            {chatBusy ? t('stop') : t('send')}
          </button>
        </div>
      </section>

      {warnings.length > 0 && (
        <section style={{ marginTop: 24 }}>
          <h3>{t('warnings_title')}</h3>
          <ul>
            {warnings.map((w) => (
              <li
                key={w.code}
                style={{
                  color: w.severity === 'block' ? '#f06060' : '#d49b3a',
                  fontSize: 13,
                  marginBottom: 4,
                }}
              >
                {t(`warning_${w.code}`)}
              </li>
            ))}
          </ul>
        </section>
      )}

      <div className="dm-wizard-action-bar">
        {commitError && <p role="alert">{t(commitError)}</p>}
        <button
          type="button"
          className="dm-wizard-btn-primary"
          disabled={blocking || committing}
          onClick={begin}
        >
          {committing
            ? t('character_committing')
            : mode === 'edit'
              ? t('replace_character')
              : t('begin_adventure')}
        </button>
      </div>

      <DmConfirmModal
        open={confirmOpen}
        message={t('confirm_replace_character')}
        onConfirm={beginConfirmed}
        onCancel={() => setConfirmOpen(false)}
      />
    </section>
  );
}
