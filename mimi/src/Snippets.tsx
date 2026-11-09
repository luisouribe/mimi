import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Snippet = { id: number; phrase: string; text: string };
type Draft = { id: number | null; phrase: string; text: string };

export default function Snippets() {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [confirmingId, setConfirmingId] = useState<number | null>(null);
  const [problem, setProblem] = useState("");

  const load = () =>
    invoke<Snippet[]>("list_snippets")
      .then(setSnippets)
      .catch((error) => setProblem(String(error)));

  useEffect(() => {
    void load();
  }, []);

  const save = async () => {
    if (!draft) return;
    setProblem("");
    try {
      await invoke("save_snippet", { id: draft.id, phrase: draft.phrase, text: draft.text });
      setDraft(null);
      await load();
    } catch (error) {
      setProblem(String(error));
    }
  };

  const remove = async (id: number) => {
    setProblem("");
    try {
      await invoke("delete_snippet", { id });
      setConfirmingId(null);
      await load();
    } catch (error) {
      setProblem(String(error));
    }
  };

  if (draft) {
    return (
      <section className="panel" aria-label={draft.id === null ? "New snippet" : "Edit snippet"}>
        <h1 className="status">{draft.id === null ? "New snippet" : "Edit snippet"}</h1>
        {problem && <p className="problem">{problem}</p>}
        <div className="settings-list">
          <label className="field">
            <span>SAY THIS</span>
            <input
              type="text"
              value={draft.phrase}
              autoFocus
              placeholder="dirección de casa"
              onChange={(event) => setDraft({ ...draft, phrase: event.target.value })}
            />
          </label>
          <p className="field-note">At least two words. Mimi replaces this phrase wherever she hears it, so choose one you don't say in normal speech.</p>
          <label className="field grow">
            <span>INSERT THIS</span>
            <textarea
              value={draft.text}
              placeholder="Your address, a link, your bio, a standard reply..."
              onChange={(event) => setDraft({ ...draft, text: event.target.value })}
            />
          </label>
          <div className="form-buttons">
            <button className="talk small" type="button" onClick={() => void save()}>
              SAVE
            </button>
            <button
              className="quit"
              type="button"
              onClick={() => {
                setProblem("");
                setDraft(null);
              }}
            >
              CANCEL
            </button>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className="panel" aria-label="Snippets">
      <h1 className="status">Snippets</h1>
      <p className="field-note">Say a phrase, and Mimi replaces it with the saved text.</p>
      <button className="talk" type="button" onClick={() => setDraft({ id: null, phrase: "", text: "" })}>
        <span>NEW SNIPPET</span>
        <svg aria-hidden="true" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M8 1v14M1 8h14" />
        </svg>
      </button>
      {problem && <p className="problem">{problem}</p>}

      {snippets.length === 0 ? (
        <p className="history-empty">
          No snippets yet. For example, save your address under "dirección de casa", then say that phrase in any app.
        </p>
      ) : (
        <ul className="history">
          {snippets.map((snippet) => (
            <li key={snippet.id}>
              <div className="entry-body">
                <p className="entry-meta snippet-phrase">"{snippet.phrase}"</p>
                <p className="entry-text snippet-text">{snippet.text}</p>
                {confirmingId === snippet.id && (
                  <p className="confirm">
                    <button type="button" className="confirm-delete" onClick={() => void remove(snippet.id)}>
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
                aria-label={`Edit ${snippet.phrase}`}
                title="Edit"
                onClick={() => {
                  setConfirmingId(null);
                  setDraft({ ...snippet });
                }}
              >
                <svg aria-hidden="true" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.6">
                  <path d="M3 15l1-4 8-8 3 3-8 8-4 1zM10.5 4.5l3 3" />
                </svg>
              </button>
              <button
                className="copy"
                type="button"
                aria-label={`Delete ${snippet.phrase}`}
                title="Delete"
                onClick={() => setConfirmingId(snippet.id)}
              >
                <svg aria-hidden="true" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.6">
                  <path d="M3 5h12M7 5V3h4v2M5 5l1 10h6l1-10" />
                </svg>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
