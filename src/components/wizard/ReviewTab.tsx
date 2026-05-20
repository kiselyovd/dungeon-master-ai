import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Compendium } from '../../api/srd';
import { useCharacterAssist } from '../../hooks/useCharacterAssist';
import type { TestChatTurn } from '../../state/charCreation';
import { useStore } from '../../state/useStore';
import type { CharacterWizardMode } from '../CharacterWizard';
import { DmConfirmModal } from '../DmConfirmModal';
import {
  computeGoldRows,
  mergeInventoryRows,
  parseEquipmentString,
  promoteIcon,
  readBackgroundStartingEquipment,
  resolveEquipmentSlots,
} from './equipmentResolver';

interface Warning {
  code: string;
  severity: 'block' | 'warn';
}

export interface ReviewTabProps {
  compendium: Compendium;
  mode: CharacterWizardMode;
  onClose?: () => void;
}

export function ReviewTab({ compendium, mode, onClose }: ReviewTabProps) {
  const { t } = useTranslation('wizard');
  const draft = useStore((s) => s.charCreation);
  const resetDraft = useStore((s) => s.charCreation.resetDraft);
  const replaceFromDraft = useStore((s) => s.pc.replaceFromDraft);
  const completeOnboarding = useStore((s) => s.onboarding.complete);
  const { runTestChat, cancel } = useCharacterAssist();

  const [history, setHistory] = useState<TestChatTurn[]>([]);
  const [userInput, setUserInput] = useState('');
  const [chatBusy, setChatBusy] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

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
    beginConfirmed();
  }

  function beginConfirmed() {
    setConfirmOpen(false);
    const bg = draft.backgroundId
      ? (compendium.backgrounds.find((b) => b.id === draft.backgroundId) ?? null)
      : null;
    const bgItems = readBackgroundStartingEquipment(bg);

    const resolvedFromSlots =
      draft.equipmentMode === 'package'
        ? resolveEquipmentSlots(draft.equipmentSlots, bgItems, compendium)
        : [];

    const fromGold =
      draft.equipmentMode === 'gold'
        ? draft.equipmentInventory.map((it) => promoteIcon(it, compendium))
        : [];

    const bgItemsForGold =
      draft.equipmentMode === 'gold' && bg ? resolveEquipmentSlots([], bgItems, compendium) : [];

    const goldRows = computeGoldRows(
      { equipmentMode: draft.equipmentMode, goldRemaining: draft.goldRemaining },
      bg,
    );

    const inventory = mergeInventoryRows([
      ...resolvedFromSlots,
      ...fromGold,
      ...bgItemsForGold,
      ...goldRows,
    ]);

    replaceFromDraft({
      heroClass: draft.classId,
      name: draft.name || 'Hero',
      race: draft.raceId,
      subclass: draft.subclassId,
      background: draft.backgroundId,
      alignment: draft.alignment,
      level: 1,
      abilities: draft.abilities,
      inventory,
    });
    resetDraft();
    completeOnboarding();
    onClose?.();
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
        <button type="button" className="dm-wizard-btn-primary" disabled={blocking} onClick={begin}>
          {mode === 'edit' ? t('replace_character') : t('begin_adventure')}
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
