import { useState } from 'react';
import type { ToolLogEntry } from '../state/toolLog';
import styles from './ToolInspectorDrawer.module.css';

interface Props {
  entries: ToolLogEntry[];
  isOpen: boolean;
  onClose: () => void;
}

/**
 * 480px right drawer showing raw tool-call request/response JSON.
 * M3: always-on for dev (production-gating in M5).
 * Per design handoff section 10: gold left border, collapsible entries,
 * copy-as-cURL per entry.
 */
export function ToolInspectorDrawer({ entries, isOpen, onClose }: Props) {
  if (!isOpen) return null;

  return (
    <aside className={styles.drawer} aria-label="Tool-call inspector">
      <div className={styles.header}>
        <span className={styles.title}>Tool-call inspector</span>
        <button
          type="button"
          className={styles.closeBtn}
          onClick={onClose}
          aria-label="Close inspector"
        >
          &#x2715;
        </button>
      </div>
      <div className={styles.list}>
        {entries.length === 0 && <p className={styles.empty}>No tool calls yet.</p>}
        {[...entries].reverse().map((entry) => (
          <InspectorEntry key={entry.id} entry={entry} />
        ))}
      </div>
    </aside>
  );
}

function InspectorEntry({ entry }: { entry: ToolLogEntry }) {
  const [expanded, setExpanded] = useState(false);
  const statusLabel = entry.result === null ? 'pending' : entry.isError ? 'fail' : 'success';

  const copyAsCurl = () => {
    const curlCmd = `curl -X POST "http://127.0.0.1:PORT/agent/turn" \\
  -H "Content-Type: application/json" \\
  -d '${JSON.stringify({ tool_name: entry.toolName, args: entry.args }, null, 2)}'`;
    navigator.clipboard.writeText(curlCmd).catch(() => undefined);
  };

  return (
    <div className={styles.entry}>
      <div className={styles.entryHeader}>
        <button
          type="button"
          className={styles.entryToggle}
          onClick={() => setExpanded((p) => !p)}
          aria-expanded={expanded}
        >
          <span className={styles.entryName}>{entry.toolName}</span>
          <span className={`${styles.entryStatus} ${styles[`status_${statusLabel}`]}`}>
            {statusLabel}
          </span>
          <span className={styles.entryTime}>{new Date(entry.timestamp).toLocaleTimeString()}</span>
        </button>
        <button
          type="button"
          className={styles.curlBtn}
          onClick={copyAsCurl}
          aria-label="Copy as cURL"
        >
          cURL
        </button>
      </div>
      {expanded && (
        <div className={styles.entryBody}>
          <div className={styles.jsonSection}>
            <span className={styles.jsonLabel}>Request</span>
            <pre className={styles.json}>{JSON.stringify(entry.args, null, 2)}</pre>
          </div>
          {entry.result !== null && (
            <div className={styles.jsonSection}>
              <span className={styles.jsonLabel}>Response</span>
              <pre className={`${styles.json} ${entry.isError ? styles.jsonError : ''}`}>
                {JSON.stringify(entry.result, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
