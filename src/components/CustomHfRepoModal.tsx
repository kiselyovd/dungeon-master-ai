import { type FormEvent, useEffect, useId, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import styles from './CustomHfRepoModal.module.css';

export interface CustomModelInput {
  hf_repo: string;
  gguf_filename: string;
  mmproj_filename?: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  onSave: (input: CustomModelInput) => void;
}

interface ValidationState {
  repoError: string | null;
  ggufError: string | null;
  mmprojError: string | null;
  valid: boolean;
}

const REPO_RE = /^[^\s/]+\/[^\s/]+$/;

export function CustomHfRepoModal({ open, onClose, onSave }: Props) {
  const { t } = useTranslation('settings');
  const [hfRepo, setHfRepo] = useState('');
  const [ggufFilename, setGgufFilename] = useState('');
  const [mmprojFilename, setMmprojFilename] = useState('');

  useEffect(() => {
    if (!open) {
      setHfRepo('');
      setGgufFilename('');
      setMmprojFilename('');
    }
  }, [open]);

  const repoId = useId();
  const ggufId = useId();
  const mmprojId = useId();
  const repoErrId = useId();
  const ggufErrId = useId();
  const mmprojErrId = useId();

  const validation: ValidationState = useMemo(() => {
    const repoTrim = hfRepo.trim();
    const ggufTrim = ggufFilename.trim();
    const mmprojTrim = mmprojFilename.trim();

    let repoError: string | null = null;
    if (repoTrim.length === 0) {
      repoError = t('custom_modal_validation_repo_required');
    } else if (!REPO_RE.test(repoTrim)) {
      repoError = t('custom_modal_validation_repo_format');
    }

    let ggufError: string | null = null;
    if (ggufTrim.length === 0) {
      ggufError = t('custom_modal_validation_gguf_required');
    } else if (!ggufTrim.toLowerCase().endsWith('.gguf')) {
      ggufError = t('custom_modal_validation_gguf_extension');
    }

    let mmprojError: string | null = null;
    if (mmprojTrim.length > 0) {
      if (!mmprojTrim.toLowerCase().endsWith('.gguf')) {
        mmprojError = t('custom_modal_validation_mmproj_extension');
      } else if (mmprojTrim === ggufTrim) {
        mmprojError = t('custom_modal_validation_mmproj_same');
      }
    }

    return {
      repoError,
      ggufError,
      mmprojError,
      valid: !repoError && !ggufError && !mmprojError,
    };
  }, [hfRepo, ggufFilename, mmprojFilename, t]);

  // Per spec: errors only render once the user has typed in the field, so
  // an empty form doesn't shout "required" before they touch anything.
  const showRepoError = hfRepo.length > 0 && validation.repoError;
  const showGgufError = ggufFilename.length > 0 && validation.ggufError;
  const showMmprojError = mmprojFilename.length > 0 && validation.mmprojError;

  const handleSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!validation.valid) return;
    const payload: CustomModelInput = {
      hf_repo: hfRepo.trim(),
      gguf_filename: ggufFilename.trim(),
    };
    const mmproj = mmprojFilename.trim();
    if (mmproj.length > 0) {
      payload.mmproj_filename = mmproj;
    }
    onSave(payload);
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t('custom_modal_title')}
      footer={
        <>
          <Button onClick={onClose}>{t('custom_modal_cancel')}</Button>
          <Button
            variant="primary"
            type="submit"
            form="custom-hf-repo-form"
            disabled={!validation.valid}
          >
            {t('custom_modal_save')}
          </Button>
        </>
      }
    >
      <form id="custom-hf-repo-form" className={styles.form} onSubmit={handleSubmit} noValidate>
        <div className={styles.fieldRow}>
          <label htmlFor={repoId} className={styles.label}>
            {t('custom_modal_hf_repo_label')}
          </label>
          <input
            id={repoId}
            type="text"
            value={hfRepo}
            onChange={(e) => setHfRepo(e.target.value)}
            placeholder={t('custom_modal_hf_repo_placeholder')}
            className={`${styles.input} ${showRepoError ? styles.inputInvalid : ''}`}
            aria-invalid={!!showRepoError}
            aria-describedby={showRepoError ? repoErrId : undefined}
          />
          {showRepoError ? (
            <span id={repoErrId} role="alert" className={styles.error}>
              {validation.repoError}
            </span>
          ) : null}
        </div>

        <div className={styles.fieldRow}>
          <label htmlFor={ggufId} className={styles.label}>
            {t('custom_modal_gguf_filename_label')}
          </label>
          <input
            id={ggufId}
            type="text"
            value={ggufFilename}
            onChange={(e) => setGgufFilename(e.target.value)}
            placeholder={t('custom_modal_gguf_filename_placeholder')}
            className={`${styles.input} ${showGgufError ? styles.inputInvalid : ''}`}
            aria-invalid={!!showGgufError}
            aria-describedby={showGgufError ? ggufErrId : undefined}
          />
          {showGgufError ? (
            <span id={ggufErrId} role="alert" className={styles.error}>
              {validation.ggufError}
            </span>
          ) : null}
        </div>

        <div className={styles.fieldRow}>
          <label htmlFor={mmprojId} className={styles.label}>
            {t('custom_modal_mmproj_filename_label')}
          </label>
          <input
            id={mmprojId}
            type="text"
            value={mmprojFilename}
            onChange={(e) => setMmprojFilename(e.target.value)}
            placeholder={t('custom_modal_mmproj_filename_placeholder')}
            className={`${styles.input} ${showMmprojError ? styles.inputInvalid : ''}`}
            aria-invalid={!!showMmprojError}
            aria-describedby={showMmprojError ? mmprojErrId : undefined}
          />
          {showMmprojError ? (
            <span id={mmprojErrId} role="alert" className={styles.error}>
              {validation.mmprojError}
            </span>
          ) : null}
        </div>
      </form>
    </Modal>
  );
}
