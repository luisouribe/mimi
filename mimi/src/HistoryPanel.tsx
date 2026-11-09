import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { HistoryEntry } from "./App";

const COPIED_TIME_MS = 1800;
const SEARCH_WAIT_MS = 120;
const SEARCH_LIMIT = 200;
/** Starts loading older dictations this many pixels before the list ends. */
const LOAD_MORE_MARGIN = 300;

function timeLabel(ms: number) {
  const date = new Date(ms);
  const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return date.toDateString() === new Date().toDateString()
    ? time
    : `${date.toLocaleDateString([], { day: "numeric", month: "short" })}, ${time}`;
}

type Props = {
  entries: HistoryEntry[];
  loadMore: () => void;
  onDeleted: (id: number) => void;
  onProblem: (message: string) => void;
};

export default function HistoryPanel({ entries, loadMore, onDeleted, onProblem }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<HistoryEntry[] | null>(null);
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const [confirmingId, setConfirmingId] = useState<number | null>(null);
  const copiedTimer = useRef(0);
  const latestSearch = useRef(0);

  useEffect(() => () => clearTimeout(copiedTimer.current), []);

  useEffect(() => {
    const words = query.trim();
    const thisSearch = ++latestSearch.current;
    if (!words) {
      setResults(null);
      return;
    }
    const timer = window.setTimeout(async () => {
      try {
        const found = await invoke<HistoryEntry[]>("history_search", { query: words });
        // A slower, older search must not replace the answer to what's typed now.
        if (thisSearch === latestSearch.current) setResults(found);
      } catch (error) {
        onProblem(String(error));
      }
    }, SEARCH_WAIT_MS);
    return () => clearTimeout(timer);
  }, [query, onProblem]);

  const copy = async (entry: HistoryEntry) => {
    try {
      await invoke("copy_text", { text: entry.text });
      setCopiedId(entry.id);
      clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopiedId(null), COPIED_TIME_MS);
    } catch (error) {
      onProblem(String(error));
    }
  };

  const remove = async (id: number) => {
    try {
      await invoke("history_delete", { id });
      setConfirmingId(null);
      setResults((current) => current && current.filter((entry) => entry.id !== id));
      onDeleted(id);
    } catch (error) {
      onProblem(String(error));
    }
  };

  const searching = results !== null;
  const shown = results ?? entries;

  return (
    <div className="history-area">
      <div className="history-head">
        <h2>HISTORY</h2>
        {(entries.length > 0 || query) && (
          <label className="search">
            <svg aria-hidden="true" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.6">
              <circle cx="6" cy="6" r="4.2" />
              <path d="M9.2 9.2L13 13" />
            </svg>
            <input
              type="text"
              value={query}
              placeholder="Search"
              aria-label="Search your dictations"
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Escape") setQuery("");
              }}
            />
            {query && (
              <button type="button" aria-label="Clear search" onClick={() => setQuery("")}>
                <svg aria-hidden="true" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.6">
                  <path d="M2 2l8 8M10 2l-8 8" />
                </svg>
              </button>
            )}
          </label>
        )}
      </div>

      {shown.length === 0 ? (
        <p className="history-empty">
          {searching ? `Nothing found for "${query.trim()}".` : "Everything you dictate is saved here, on this PC only."}
        </p>
      ) : (
        <ul
          className="history"
          onScroll={(event) => {
            const list = event.currentTarget;
            if (!searching && list.scrollHeight - list.scrollTop - list.clientHeight < LOAD_MORE_MARGIN) loadMore();
          }}
        >
          {shown.map((entry) => (
            <li key={entry.id}>
              <div className="entry-body">
                <p className="entry-meta">
                  {entry.app_name} / {timeLabel(entry.created_at_ms)}
                </p>
                <p className="entry-text">{entry.text}</p>
                {confirmingId === entry.id && (
                  <p className="confirm">
                    <button type="button" className="confirm-delete" onClick={() => void remove(entry.id)}>
                      DELETE
                    </button>
                    <button type="button" className="confirm-keep" onClick={() => setConfirmingId(null)}>
                      KEEP
                    </button>
                  </p>
                )}
              </div>
              <button
                className="copy"
                type="button"
                onClick={() => void copy(entry)}
                aria-label={copiedId === entry.id ? "Copied" : "Copy this text"}
                title="Copy"
              >
                {copiedId === entry.id ? (
                  <svg aria-hidden="true" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="#c22b38" strokeWidth="2.2">
                    <path d="M3 9.5l4 4 8-9" />
                  </svg>
                ) : (
                  <svg aria-hidden="true" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.6">
                    <rect x="6" y="6" width="9" height="10" />
                    <path d="M12 6V2.5H3V13h3" />
                  </svg>
                )}
              </button>
              <button
                className="copy"
                type="button"
                aria-label="Delete this dictation"
                title="Delete"
                onClick={() => setConfirmingId(entry.id)}
              >
                <svg aria-hidden="true" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.6">
                  <path d="M3 5h12M7 5V3h4v2M5 5l1 10h6l1-10" />
                </svg>
              </button>
            </li>
          ))}
        </ul>
      )}
      {results !== null && results.length === SEARCH_LIMIT && (
        <p className="field-note">Showing the newest {SEARCH_LIMIT} matches. Add a word to narrow them down.</p>
      )}
    </div>
  );
}
