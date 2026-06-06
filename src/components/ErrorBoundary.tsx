import i18next from 'i18next';
import React, { Component, type ErrorInfo, type ReactNode } from 'react';
import { useStore } from '../state/useStore';

/**
 * Translate via the i18next singleton with an English fallback. ErrorBoundary
 * is a class component (no hooks), and i18n may not be initialised yet when an
 * early crash renders the boundary - hence the guard. [M11 G4]
 */
function tr(key: string, fallback: string): string {
  return i18next.isInitialized ? i18next.t(key) : fallback;
}

export type ErrorBoundaryLevel = 'top' | 'section' | 'overlay';

interface Props {
  level: ErrorBoundaryLevel;
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  /** Incremented on Retry to force a full remount of children. */
  remountKey: number;
}

function buildCrashReport(error: Error, errorInfo: ErrorInfo): string {
  const state = useStore.getState();
  const lastMessages = state.chat.messages.slice(-3).map((m) => ({
    role: m.role,
    content: m.content.slice(0, 200),
  }));
  const combat = {
    active: state.combat.active,
    round: state.combat.round,
    currentTurnId: state.combat.currentTurnId,
    tokens: state.combat.tokens.map((t) => ({ id: t.id, name: t.name, hp: t.hp })),
  };

  const payload = {
    version: __APP_VERSION__,
    error: error.message,
    stack: error.stack ?? '',
    componentStack: errorInfo.componentStack ?? '',
    lastMessages,
    combat,
  };

  return JSON.stringify(payload, null, 2);
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      remountKey: 0,
    };
  }

  static getDerivedStateFromError(error: Error): Pick<State, 'hasError' | 'error'> {
    return { hasError: true, error };
  }

  override componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    const { level } = this.props;

    if (level === 'overlay') {
      // Auto-recover: log only, then clear the error state
      console.error('[ErrorBoundary overlay] caught and recovering:', error, errorInfo);
      this.setState({ hasError: false, error: null, errorInfo: null });
      return;
    }

    this.setState({ errorInfo });
    console.error(`[ErrorBoundary ${level}] caught:`, error, errorInfo);
  }

  private handleReload = (): void => {
    window.location.reload();
  };

  private handleCopy = (): void => {
    const { error, errorInfo } = this.state;
    if (!error || !errorInfo) return;
    const report = buildCrashReport(error, errorInfo);
    void navigator.clipboard.writeText(report);
  };

  private handleRetry = (): void => {
    this.setState((prev) => ({
      hasError: false,
      error: null,
      errorInfo: null,
      remountKey: prev.remountKey + 1,
    }));
  };

  override render(): ReactNode {
    const { hasError, error, remountKey } = this.state;
    const { level, children } = this.props;

    if (!hasError) {
      return <React.Fragment key={remountKey}>{children}</React.Fragment>;
    }

    if (level === 'overlay') {
      // componentDidCatch already cleared the error state; render nothing in the interim
      return null;
    }

    if (level === 'top') {
      return (
        <div className="dm-error-boundary dm-error-boundary-top" data-testid="error-boundary-top">
          <div className="dm-error-boundary-card">
            <h2 className="dm-error-boundary-title">
              {tr('common:error_title', 'Something went wrong')}
            </h2>
            <p className="dm-error-boundary-message">{error?.message}</p>
            <div className="dm-error-boundary-actions">
              <button
                type="button"
                className="dm-btn dm-btn-primary"
                onClick={this.handleReload}
                aria-label={tr('common:error_reload', 'Reload')}
              >
                {tr('common:error_reload', 'Reload')}
              </button>
              <button
                type="button"
                className="dm-btn"
                onClick={this.handleCopy}
                aria-label={tr('common:error_copy_report', 'Copy crash report')}
              >
                {tr('common:error_copy_report', 'Copy crash report')}
              </button>
            </div>
          </div>
        </div>
      );
    }

    if (level === 'section') {
      return (
        <div
          className="dm-error-boundary dm-error-boundary-section"
          data-testid="error-boundary-section"
        >
          <div className="dm-error-boundary-card">
            <p className="dm-error-boundary-message">{error?.message}</p>
            <button
              type="button"
              className="dm-btn dm-btn-primary"
              onClick={this.handleRetry}
              aria-label={tr('common:error_retry', 'Retry')}
            >
              {tr('common:error_retry', 'Retry')}
            </button>
          </div>
        </div>
      );
    }

    // Should be unreachable: overlay is handled above, top and section are handled above.
    return null;
  }
}
