import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { PreflightModal } from '../PreflightModal';

/**
 * PreflightModal - unit tests covering per-status behaviour and CTA callbacks.
 *
 * Issue 1 regression: the missing_chat "Finish setup" CTA must call
 * onFinishSetup (which in App.tsx now opens Settings on the Chat tab),
 * NOT reset onboarding. This test is the canary.
 */

describe('PreflightModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  // -------------------------------------------------------------------------
  // missing_chat: blocking modal - no dismiss button, Finish setup CTA fires
  // -------------------------------------------------------------------------
  describe('missing_chat', () => {
    it('renders without a dismiss button (blocking)', () => {
      render(<PreflightModal status="missing_chat" onFinishSetup={vi.fn()} onDismiss={vi.fn()} />);
      expect(screen.queryByTestId('preflight-dismiss')).not.toBeInTheDocument();
    });

    it('calls onFinishSetup when "Finish setup" is clicked', async () => {
      const user = userEvent.setup();
      const onFinishSetup = vi.fn();
      render(
        <PreflightModal status="missing_chat" onFinishSetup={onFinishSetup} onDismiss={vi.fn()} />,
      );
      await user.click(screen.getByTestId('preflight-finish-setup'));
      expect(onFinishSetup).toHaveBeenCalledOnce();
    });

    it('does NOT call onDismiss when "Finish setup" is clicked (chat is blocking)', async () => {
      const user = userEvent.setup();
      const onDismiss = vi.fn();
      const onFinishSetup = vi.fn();
      render(
        <PreflightModal
          status="missing_chat"
          onFinishSetup={onFinishSetup}
          onDismiss={onDismiss}
        />,
      );
      await user.click(screen.getByTestId('preflight-finish-setup'));
      // onFinishSetup fires; onDismiss should NOT fire via the Finish button
      expect(onFinishSetup).toHaveBeenCalledOnce();
      expect(onDismiss).not.toHaveBeenCalled();
    });
  });

  // -------------------------------------------------------------------------
  // missing_image: non-blocking - dismiss button and checkbox present
  // -------------------------------------------------------------------------
  describe('missing_image', () => {
    it('renders the dismiss button', () => {
      render(<PreflightModal status="missing_image" onFinishSetup={vi.fn()} onDismiss={vi.fn()} />);
      expect(screen.getByTestId('preflight-dismiss')).toBeInTheDocument();
    });

    it('calls onDismiss when dismiss is clicked', async () => {
      const user = userEvent.setup();
      const onDismiss = vi.fn();
      render(
        <PreflightModal status="missing_image" onFinishSetup={vi.fn()} onDismiss={onDismiss} />,
      );
      await user.click(screen.getByTestId('preflight-dismiss'));
      expect(onDismiss).toHaveBeenCalledOnce();
    });

    it('calls onFinishSetup when "Finish setup" is clicked', async () => {
      const user = userEvent.setup();
      const onFinishSetup = vi.fn();
      render(
        <PreflightModal status="missing_image" onFinishSetup={onFinishSetup} onDismiss={vi.fn()} />,
      );
      await user.click(screen.getByTestId('preflight-finish-setup'));
      expect(onFinishSetup).toHaveBeenCalledOnce();
    });
  });

  // -------------------------------------------------------------------------
  // missing_video: non-blocking - same affordances as missing_image
  // -------------------------------------------------------------------------
  describe('missing_video', () => {
    it('renders the dismiss button', () => {
      render(<PreflightModal status="missing_video" onFinishSetup={vi.fn()} onDismiss={vi.fn()} />);
      expect(screen.getByTestId('preflight-dismiss')).toBeInTheDocument();
    });

    it('calls onFinishSetup when "Finish setup" is clicked', async () => {
      const user = userEvent.setup();
      const onFinishSetup = vi.fn();
      render(
        <PreflightModal status="missing_video" onFinishSetup={onFinishSetup} onDismiss={vi.fn()} />,
      );
      await user.click(screen.getByTestId('preflight-finish-setup'));
      expect(onFinishSetup).toHaveBeenCalledOnce();
    });
  });
});
