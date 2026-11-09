import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "@fontsource-variable/oxanium";
import "@fontsource-variable/manrope";
import { startMimiStage, type MimiPhase, type MimiStage } from "./mimiStage";
import { runSelfTest, type SelfTestPlan } from "./selfTest";
import HistoryPanel from "./HistoryPanel";
import Snippets from "./Snippets";
import { MODEL_LANGUAGES } from "./languages";
import "./App.css";

export type ModelChoice = "parakeet" | "whisper-turbo" | "whisper-small";
export type EngineStatus =
  | { state: "loading"; model: ModelChoice }
  | {
      state: "ready";
      model: ModelChoice;
      load_seconds: number;
      uses_graphics_card: boolean;
      problem: string | null;
    }
  | { state: "failed"; message: string };
type ModelInfo = {
  id: ModelChoice;
  name: string;
  description: string;
  installed: boolean;
};
export type Shortcut = {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  key: number;
  key_name: string;
};
type Settings = {
  model: ModelChoice;
  language: string | null;
  whisper_cpu_only: boolean;
  cleanup: boolean;
  animate_mimi: boolean;
  start_with_windows: boolean;
  chimes: boolean;
  quiet_other_apps: boolean;
  hold_shortcut: Shortcut;
  tap_shortcut: Shortcut;
  appearance: Appearance;
};
type Appearance = "light" | "dark" | "system";
type Mic = { name: string; is_default: boolean };
type Phase = "idle" | "opening" | "listening" | "working";
type Stage = { phase: Phase } | { phase: "notice"; message: string };
export type HistoryEntry = {
  id: number;
  text: string;
  app_name: string;
  created_at_ms: number;
  word_count: number;
  speech_seconds: number;
  pasted: boolean;
};
export type Dictation = {
  text: string;
  heard_speech: boolean;
  word_count: number;
  entry: HistoryEntry | null;
  timings: {
    mic_ready_ms: number | null;
    recorded_seconds: number;
    speech_seconds: number;
    chunk_count: number;
    cleanup_ms: number;
    recognition_ms: number;
    tidy_ms: number;
    stop_to_text_ms: number;
    paste_ms: number;
    total_ms: number;
    outcome: string;
  };
};

const MIC_STORAGE_KEY = "mimi-microphone";
const HISTORY_PAGE = 100;
const RESTING_BAR = 0.14;
const BAR_HEIGHTS = [16, 26, 38, 22, 44, 30, 18, 36, 44, 24, 14, 32, 42, 20, 34, 26, 12, 28, 40, 18];
const LANGUAGES: [string, string][] = [
  ["ja", "Japanese"],
  ["es", "Spanish"],
  ["en", "English"],
  ["pt", "Portuguese"],
  ["fr", "French"],
  ["de", "German"],
  ["it", "Italian"],
  ["zh", "Chinese"],
  ["ko", "Korean"],
  ["ru", "Russian"],
  ["ar", "Arabic"],
  ["hi", "Hindi"],
];

function shortcutKeys(shortcut: Shortcut) {
  return [shortcut.ctrl && "Ctrl", shortcut.alt && "Alt", shortcut.shift && "Shift", shortcut.key_name].filter(
    (key): key is string => Boolean(key),
  );
}

export function shortcutLabel(shortcut: Shortcut) {
  return shortcutKeys(shortcut).join(" + ");
}

function Keys({ shortcut }: { shortcut: Shortcut }) {
  return (
    <span className="keys">
      {shortcutKeys(shortcut).map((key, index) => (
        <span key={key} className="key-part">
          {index > 0 && <span aria-hidden="true">+</span>}
          <kbd>{key}</kbd>
        </span>
      ))}
    </span>
  );
}

const APPEARANCE_STORAGE_KEY = "mimi-appearance";

function isDark(appearance: Appearance) {
  return (
    appearance === "dark" || (appearance === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches)
  );
}

/** Puts the chosen colors on the page. "system" follows Windows' light or dark app mode. */
function applyAppearance(appearance: Appearance) {
  const dark =
    appearance === "dark" || (appearance === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  try {
    localStorage.setItem(APPEARANCE_STORAGE_KEY, appearance);
  } catch {
    // Remembering it only avoids a flash of the light colors at startup.
  }
}

function readSavedMic() {
  try {
    return localStorage.getItem(MIC_STORAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

function languageLabel(settings: Settings) {
  if (settings.model === "parakeet") return "Español / English";
  const chosen = LANGUAGES.find(([code]) => code === settings.language);
  return chosen ? chosen[1] : "Detected automatically";
}

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const barsRef = useRef<(HTMLSpanElement | null)[]>([]);
  const stagePhaseRef = useRef<MimiPhase>("idle");
  const voiceRef = useRef(0);
  const loadingMore = useRef(false);
  const hasMore = useRef(true);
  const enginePoll = useRef(0);
  const windowShown = useRef(false);
  const [phase, setPhase] = useState<Phase>("idle");
  const [animated, setAnimated] = useState(false);
  const [engine, setEngine] = useState<EngineStatus>({
    state: "loading",
    model: "parakeet",
  });
  const [view, setView] = useState<"home" | "snippets" | "settings">("home");
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [mics, setMics] = useState<Mic[]>([]);
  const [mic, setMic] = useState(readSavedMic);
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const entriesRef = useRef(entries);
  entriesRef.current = entries;
  const [problem, setProblem] = useState("");
  /** How many dictations a "delete all" would remove, while asking to confirm. */
  const [clearCount, setClearCount] = useState<number | null>(null);
  /** Which shortcut is waiting for new keys. */
  const [changing, setChanging] = useState<"hold" | "tap" | null>(null);
  const [shortcutProblem, setShortcutProblem] = useState("");
  const [maximized, setMaximized] = useState(false);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const changingRef = useRef(changing);
  changingRef.current = changing;

  stagePhaseRef.current = phase === "listening" ? "listening" : phase === "working" ? "working" : "idle";
  const animateMimi = settings?.animate_mimi;

  useEffect(() => {
    if (animateMimi === undefined) return;
    let stage: MimiStage | undefined;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    const showWindow = () => {
      if (windowShown.current) return;
      windowShown.current = true;
      void invoke("window_ready");
    };

    // Runs even when Windows animation effects are off. The Settings switch is what turns it off.
    async function start() {
      const started = await startMimiStage(
        canvasRef.current!,
        "/mimi.png",
        "/mimi-motion-map.png",
        () => stagePhaseRef.current,
        () => voiceRef.current,
      );
      if (cancelled) {
        started.stop();
        return;
      }
      stage = started;
      setAnimated(true);

      // Mimi only moves while her window is the active one, so she costs nothing
      // while someone is dictating into another app.
      const appWindow = getCurrentWindow();
      started.setActive(await appWindow.isFocused());
      unlisten = await appWindow.onFocusChanged(({ payload }) => started.setActive(payload));
    }

    if (animateMimi) {
      start()
        .catch((error) => console.warn("Mimi animation unavailable, showing the still image", error))
        .finally(showWindow);
    } else {
      setAnimated(false);
      showWindow();
    }
    return () => {
      cancelled = true;
      unlisten?.();
      stage?.stop();
    };
  }, [animateMimi]);

  useEffect(() => {
    invoke<SelfTestPlan | null>("self_test_plan").then((plan) => {
      if (plan) void runSelfTest(plan);
    });
    invoke<Mic[]>("list_microphones")
      .then(setMics)
      .catch((error) => setProblem(String(error)));
    invoke("set_microphone", { device: readSavedMic() || null });
    invoke<HistoryEntry[]>("history_recent", { beforeId: null })
      .then((first) => {
        hasMore.current = first.length === HISTORY_PAGE;
        setEntries(first);
      })
      .catch((error) => setProblem(String(error)));

    const listeners = [
      listen<number>("mic-level", ({ payload }) => {
        const voice = Math.min(1, Math.sqrt(payload) * 2.2);
        voiceRef.current = voice;
        // The bars skip React, because the microphone reports its level many times a second.
        barsRef.current.forEach((bar, index) => {
          if (!bar) return;
          const ripple = 0.65 + 0.35 * Math.sin(performance.now() / 90 + index * 1.7);
          bar.style.transform = `scaleY(${Math.max(RESTING_BAR, voice * ripple)})`;
        });
      }),
      listen<Stage>("dictation-stage", ({ payload }) => {
        if (payload.phase === "notice") {
          setPhase("idle");
        } else {
          setPhase(payload.phase);
        }
        if (payload.phase !== "listening") {
          voiceRef.current = 0;
          barsRef.current.forEach((bar) => {
            if (bar) bar.style.transform = "";
          });
        }
      }),
      listen<HistoryEntry>("history-added", ({ payload }) => {
        setEntries((current) => [payload, ...current]);
      }),
      listen<string>("history-problem", ({ payload }) => setProblem(payload)),
      listen<string>("shortcut-problem", ({ payload }) => setProblem(payload)),
    ];
    invoke<ModelInfo[]>("list_models").then(setModels);
    invoke<Settings>("get_settings").then(setSettings);
    watchEngine();
    return () => {
      clearInterval(enginePoll.current);
      listeners.forEach((listener) => listener.then((remove) => remove()));
    };
  }, []);

  // Asks about the engine only while a model is loading, so it costs nothing the rest of the time.
  const watchEngine = () => {
    clearInterval(enginePoll.current);
    enginePoll.current = window.setInterval(async () => {
      const status = await invoke<EngineStatus>("engine_status");
      setEngine(status);
      if (status.state !== "loading") clearInterval(enginePoll.current);
    }, 300);
  };

  const changeSettings = async (model: ModelChoice, language: string | null) => {
    setProblem("");
    try {
      const saved = await invoke<Settings>("change_settings", {
        model,
        language,
      });
      setSettings(saved);
      setEngine(await invoke<EngineStatus>("engine_status"));
      watchEngine();
    } catch (error) {
      setProblem(String(error));
    }
  };

  const changeSwitch = async (
    command: "set_cleanup" | "set_animate_mimi" | "set_start_with_windows" | "set_chimes" | "set_quiet_other_apps",
    enabled: boolean,
  ) => {
    setProblem("");
    try {
      setSettings(await invoke<Settings>(command, { enabled }));
    } catch (error) {
      setProblem(String(error));
    }
  };

  const changeShortcut = async (which: "hold" | "tap") => {
    setShortcutProblem("");
    setChanging(which);
    try {
      const saved = await invoke<Settings | null>("change_shortcut", { which });
      if (saved) setSettings(saved);
    } catch (error) {
      setShortcutProblem(String(error));
    } finally {
      setChanging(null);
    }
  };

  const resetShortcuts = async () => {
    setShortcutProblem("");
    try {
      setSettings(await invoke<Settings>("reset_shortcuts"));
    } catch (error) {
      setShortcutProblem(String(error));
    }
  };

  // A backup for when Mimi's own window is in front: if the shortcut keys reach this screen
  // instead of the keyboard hook, they're passed on here. When the hook does catch them, it
  // hides them, so they never arrive here twice.
  useEffect(() => {
    const MODIFIER_KEYS = new Set([16, 17, 18, 91, 92]);
    let holding = false;
    let handledKey: number | null = null;
    const matches = (shortcut: Shortcut, event: KeyboardEvent) =>
      event.keyCode === shortcut.key &&
      event.ctrlKey === shortcut.ctrl &&
      event.altKey === shortcut.alt &&
      event.shiftKey === shortcut.shift &&
      !event.metaKey;
    const onDown = (event: KeyboardEvent) => {
      const current = settingsRef.current;
      if (!current) return;
      if (changingRef.current) {
        if (MODIFIER_KEYS.has(event.keyCode)) return;
        event.preventDefault();
        event.stopPropagation();
        handledKey = event.keyCode;
        if (event.key === "Escape" && !event.ctrlKey && !event.altKey && !event.shiftKey) {
          void invoke("cancel_shortcut_change");
          return;
        }
        void invoke("shortcut_captured_from_screen", {
          ctrl: event.ctrlKey,
          alt: event.altKey,
          shift: event.shiftKey,
          win: event.metaKey,
          key: event.keyCode,
        });
        return;
      }
      const hold = matches(current.hold_shortcut, event);
      const tap = !hold && matches(current.tap_shortcut, event);
      if (!hold && !tap) return;
      event.preventDefault();
      event.stopPropagation();
      handledKey = event.keyCode;
      if (event.repeat) return;
      holding = hold;
      void invoke("shortcut_from_screen", {
        action: hold ? "hold-down" : "tap-down",
      });
    };
    const onUp = (event: KeyboardEvent) => {
      // The main key's release must not press whatever button has focus.
      if (event.keyCode === handledKey) {
        event.preventDefault();
        event.stopPropagation();
        handledKey = null;
      }
      const current = settingsRef.current;
      if (!holding || !current) return;
      const shortcut = current.hold_shortcut;
      const released =
        event.keyCode === shortcut.key ||
        (shortcut.ctrl && event.keyCode === 17) ||
        (shortcut.alt && event.keyCode === 18) ||
        (shortcut.shift && event.keyCode === 16);
      if (!released) return;
      holding = false;
      void invoke("shortcut_from_screen", { action: "hold-up" });
    };
    window.addEventListener("keydown", onDown, true);
    window.addEventListener("keyup", onUp, true);
    return () => {
      window.removeEventListener("keydown", onDown, true);
      window.removeEventListener("keyup", onUp, true);
    };
  }, []);

  const appearance = settings?.appearance;
  useEffect(() => {
    if (!appearance) return;
    applyAppearance(appearance);
    if (appearance !== "system") return;
    const windowsMode = window.matchMedia("(prefers-color-scheme: dark)");
    const follow = () => applyAppearance("system");
    windowsMode.addEventListener("change", follow);
    return () => windowsMode.removeEventListener("change", follow);
  }, [appearance]);

  const changeAppearance = async (choice: Appearance) => {
    setProblem("");
    try {
      setSettings(await invoke<Settings>("set_appearance", { appearance: choice }));
    } catch (error) {
      setProblem(String(error));
    }
  };

  // The maximize button shows "restore" while the window fills the screen.
  useEffect(() => {
    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    const check = () => {
      appWindow
        .isMaximized()
        .then(setMaximized)
        .catch(() => {});
    };
    check();
    appWindow
      .onResized(check)
      .then((remove) => {
        unlisten = remove;
      })
      .catch(() => {});
    return () => unlisten?.();
  }, []);

  const askToClear = async () => {
    setProblem("");
    try {
      setClearCount(await invoke<number>("history_count"));
    } catch (error) {
      setProblem(String(error));
    }
  };

  const clearHistory = async () => {
    try {
      await invoke<number>("history_clear");
      hasMore.current = false;
      setEntries([]);
      setClearCount(null);
    } catch (error) {
      setProblem(String(error));
    }
  };

  const chooseMic = (name: string) => {
    setMic(name);
    void invoke("set_microphone", { device: name || null });
    try {
      localStorage.setItem(MIC_STORAGE_KEY, name);
    } catch {
      // Not remembering the choice is harmless.
    }
  };

  const toggle = useCallback(
    async (event: React.MouseEvent<HTMLButtonElement>) => {
      // A click with no mouse behind it came from the keyboard. The shortcut keys must never press
      // this button, so only Enter, never Space, can press it from the keyboard.
      const byKeyboard = event.detail === 0;
      void invoke("note_from_screen", {
        text: `talk button pressed by ${byKeyboard ? "keyboard" : "mouse"} while ${phase}`,
      });
      // Leaving focus on the button would let a stray key press it again.
      if (!byKeyboard) event.currentTarget.blur();
      setProblem("");
      try {
        if (phase === "idle") {
          await invoke("start_recording", { device: null });
        } else if (phase === "opening" || phase === "listening") {
          await invoke<Dictation>("stop_recording");
        }
      } catch (error) {
        setProblem(String(error));
      }
    },
    [phase],
  );

  const loadMore = useCallback(async () => {
    if (loadingMore.current || !hasMore.current) return;
    const oldest = entriesRef.current[entriesRef.current.length - 1];
    if (!oldest) return;
    loadingMore.current = true;
    try {
      const older = await invoke<HistoryEntry[]>("history_recent", {
        beforeId: oldest.id,
      });
      hasMore.current = older.length === HISTORY_PAGE;
      setEntries((current) => [...current, ...older]);
    } catch (error) {
      setProblem(String(error));
    } finally {
      loadingMore.current = false;
    }
  }, []);

  const onDeleted = useCallback((id: number) => {
    setEntries((current) => current.filter((entry) => entry.id !== id));
  }, []);

  const statusWord = {
    idle: engine.state === "loading" ? "Loading" : "Ready",
    opening: "Get ready",
    listening: "Listening",
    working: "Typing",
  }[phase];
  const buttonLabel = phase === "idle" ? "START TALKING" : phase === "working" ? "TYPING" : "STOP AND TYPE";
  const modelName = settings && models.find((item) => item.id === settings.model)?.name;

  const appWindow = getCurrentWindow();
  const problems = (
    <>
      {problem && <p className="problem">{problem}</p>}
      {engine.state === "failed" && <p className="problem">{engine.message}</p>}
      {engine.state === "ready" && engine.problem && <p className="problem">{engine.problem}</p>}
    </>
  );

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          <span className="brand-name" data-tauri-drag-region>
            mimi
          </span>
          <span className="brand-kana" data-tauri-drag-region>
            耳
          </span>
        </div>
        <div className="window-buttons">
          <button type="button" aria-label="Minimize window" title="Minimize" onClick={() => void appWindow.minimize()}>
            <svg
              aria-hidden="true"
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
            >
              <path d="M2 7h10" />
            </svg>
          </button>
          <button
            type="button"
            aria-label={maximized ? "Restore window size" : "Maximize window"}
            title={maximized ? "Restore" : "Maximize"}
            onClick={() => void appWindow.toggleMaximize()}
          >
            {maximized ? (
              <svg
                aria-hidden="true"
                width="14"
                height="14"
                viewBox="0 0 14 14"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.4"
              >
                <path d="M2.5 4.5h7v7h-7zM4.5 4.5v-2h7v7h-2" />
              </svg>
            ) : (
              <svg
                aria-hidden="true"
                width="14"
                height="14"
                viewBox="0 0 14 14"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.4"
              >
                <path d="M2.5 2.5h9v9h-9z" />
              </svg>
            )}
          </button>
          <button
            type="button"
            aria-label="Hide Mimi to the system tray"
            title={`Hide to the system tray. Mimi keeps listening for ${settings ? shortcutLabel(settings.hold_shortcut) : "your shortcut"}.`}
            onClick={() => void appWindow.hide()}
          >
            <svg
              aria-hidden="true"
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
            >
              <path d="M3 3l8 8M11 3l-8 8" />
            </svg>
          </button>
        </div>
      </header>

      <div className="columns">
        <aside className="side">
          <nav aria-label="Main" className="nav">
            <button type="button" aria-current={view === "home" ? "page" : undefined} onClick={() => setView("home")}>
              HOME
            </button>
            <button
              type="button"
              aria-current={view === "snippets" ? "page" : undefined}
              onClick={() => setView("snippets")}
            >
              SNIPPETS
            </button>
            <button
              type="button"
              aria-current={view === "settings" ? "page" : undefined}
              onClick={() => setView("settings")}
            >
              SETTINGS
            </button>
          </nav>

          <div className="side-bottom">
            {settings && (
              <button
                type="button"
                role="switch"
                className="theme-switch"
                aria-checked={isDark(settings.appearance)}
                aria-label="Dark mode"
                title={isDark(settings.appearance) ? "Switch to light mode" : "Switch to dark mode"}
                onClick={() => void changeAppearance(isDark(settings.appearance) ? "light" : "dark")}
              >
                <span className={isDark(settings.appearance) ? "icon" : "icon on"} aria-hidden="true">
                  <svg
                    width="20"
                    height="20"
                    viewBox="0 0 20 20"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.8"
                    strokeLinecap="round"
                  >
                    <circle cx="10" cy="10" r="3.6" fill="currentColor" />
                    <path d="M10 1.8v2.2M10 16v2.2M1.8 10H4M16 10h2.2M4.2 4.2l1.6 1.6M14.2 14.2l1.6 1.6M4.2 15.8l1.6-1.6M14.2 5.8l1.6-1.6" />
                  </svg>
                </span>
                <span className="theme-track" aria-hidden="true">
                  <span className="theme-knob" />
                </span>
                <span className={isDark(settings.appearance) ? "icon on" : "icon"} aria-hidden="true">
                  <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M15.6 12.9A6.6 6.6 0 0 1 7.1 4.4a6.6 6.6 0 1 0 8.5 8.5Z" />
                  </svg>
                </span>
              </button>
            )}
            {settings && (
              <dl className="facts">
                <div>
                  <dt>LANGUAGE</dt>
                  <dd>{languageLabel(settings)}</dd>
                </div>
                <div>
                  <dt>MODEL</dt>
                  <dd>{modelName ?? ""}</dd>
                </div>
                <div>
                  <dt>AUDIO</dt>
                  <dd>Offline, stays on this PC</dd>
                </div>
              </dl>
            )}
          </div>
        </aside>

        <section className="stage" aria-label="Mimi">
          <div className="band" aria-hidden="true" />
          <div className="word" aria-hidden="true">
            mimi
          </div>
          <div className="kana-mark" aria-hidden="true">
            耳<small>mimi</small>
          </div>
          <div className={phase === "listening" ? "ring on" : "ring"} aria-hidden="true" />
          <img className={animated ? "mimi hidden" : "mimi"} src="/mimi.png" alt="Mimi, smiling" />
          <canvas className={animated ? "mimi-live" : "mimi-live hidden"} ref={canvasRef} aria-hidden="true" />
        </section>

        {view === "home" ? (
          <section className="panel" aria-label="Dictation">
            <div className="controls">
              <div className="status-row">
                <p className="status" role="status">
                  {statusWord}
                </p>
                <div className={phase === "listening" ? "wave on" : "wave"} aria-hidden="true">
                  {BAR_HEIGHTS.map((height, index) => (
                    <span
                      key={index}
                      ref={(element) => {
                        barsRef.current[index] = element;
                      }}
                      style={{ height }}
                    />
                  ))}
                </div>
              </div>

              {settings && (
                <div className="shortcuts">
                  <p className="shortcut">
                    <Keys shortcut={settings.hold_shortcut} />
                    <span className="hint">{phase === "listening" ? "Release to finish" : "Hold to talk"}</span>
                  </p>
                  <p className="shortcut">
                    <Keys shortcut={settings.tap_shortcut} />
                    <span className="hint">
                      {phase === "listening" ? "Press again to finish" : "Tap to start, again to stop"}
                    </span>
                  </p>
                </div>
              )}

              <button
                className="talk"
                type="button"
                onClick={toggle}
                onKeyDown={(event) => {
                  if (event.key === " ") event.preventDefault();
                }}
                onKeyUp={(event) => {
                  if (event.key === " ") event.preventDefault();
                }}
                disabled={phase === "working"}
              >
                <span>{buttonLabel}</span>
                <svg
                  aria-hidden="true"
                  width="30"
                  height="14"
                  viewBox="0 0 30 14"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M0 7h28M22 1l6 6-6 6" />
                </svg>
              </button>
            </div>

            {problems}

            <HistoryPanel entries={entries} loadMore={loadMore} onDeleted={onDeleted} onProblem={setProblem} />
          </section>
        ) : view === "snippets" ? (
          <Snippets />
        ) : (
          <section className="panel settings" aria-label="Settings">
            <h1 className="status">Settings</h1>
            {problems}
            <div className="settings-list">
              {settings && (
                <>
                  <label className="field">
                    <span>SPEECH MODEL</span>
                    <select
                      value={settings.model}
                      disabled={phase !== "idle" || engine.state === "loading"}
                      onChange={(event) => {
                        const model = event.target.value as ModelChoice;
                        void changeSettings(model, model === "parakeet" ? null : settings.language);
                      }}
                    >
                      {models.map((item) => (
                        <option key={item.id} value={item.id} disabled={!item.installed}>
                          {item.installed ? item.name : `${item.name} (not installed)`}
                        </option>
                      ))}
                    </select>
                  </label>
                  <p className="field-note">{models.find((item) => item.id === settings.model)?.description}</p>
                  <div className="languages">
                    <span className="languages-title">LANGUAGES EACH MODEL UNDERSTANDS</span>
                    {MODEL_LANGUAGES.map((model) => (
                      <details key={model.id}>
                        <summary>
                          <strong>{model.name}:</strong> {model.summary}
                        </summary>
                        <p>{model.languages.join(", ")}.</p>
                      </details>
                    ))}
                    <p className="field-note">
                      Whisper is most accurate in widely spoken languages, and much less so in rare ones.
                    </p>
                  </div>
                  {engine.state === "loading" && (
                    <p className="field-note">
                      Preparing the model. The first time on a graphics card can take about 30 seconds.
                    </p>
                  )}
                  {settings.whisper_cpu_only && settings.model !== "parakeet" && (
                    <p className="field-note">
                      Whisper is using the processor, because the graphics card had a problem.
                    </p>
                  )}

                  <label className="field">
                    <span>LANGUAGE</span>
                    <select
                      value={settings.language ?? ""}
                      disabled={settings.model === "parakeet" || phase !== "idle"}
                      onChange={(event) => void changeSettings(settings.model, event.target.value || null)}
                    >
                      <option value="">Detect automatically</option>
                      {LANGUAGES.map(([code, name]) => (
                        <option key={code} value={code}>
                          {name}
                        </option>
                      ))}
                    </select>
                  </label>
                  {settings.model === "parakeet" && (
                    <p className="field-note">For Japanese, choose a Whisper model first.</p>
                  )}
                  {settings.model !== "parakeet" && !settings.language && (
                    <p className="field-note">Choosing your language gives much better punctuation.</p>
                  )}
                </>
              )}

              <label className="field">
                <span>MICROPHONE</span>
                <select value={mic} onChange={(event) => chooseMic(event.target.value)} disabled={phase !== "idle"}>
                  <option value="">Windows default</option>
                  {mics.map((item) => (
                    <option key={item.name} value={item.name}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </label>

              {settings && (
                <>
                  <div className="field shortcut-settings">
                    <span>SHORTCUTS</span>
                    {(["hold", "tap"] as const).map((which) => (
                      <div className="shortcut-row" key={which}>
                        <div className="shortcut-what">
                          <span className="shortcut-name">
                            {which === "hold" ? "Hold to talk" : "Tap to start and stop"}
                          </span>
                          {changing === which ? (
                            <span className="shortcut-waiting" role="status">
                              Press the new keys now. Esc cancels.
                            </span>
                          ) : (
                            <Keys shortcut={which === "hold" ? settings.hold_shortcut : settings.tap_shortcut} />
                          )}
                        </div>
                        {changing === which ? (
                          <button
                            type="button"
                            className="small-button"
                            onClick={() => void invoke("cancel_shortcut_change")}
                          >
                            CANCEL
                          </button>
                        ) : (
                          <button
                            type="button"
                            className="small-button"
                            disabled={changing !== null || phase !== "idle"}
                            onClick={() => void changeShortcut(which)}
                          >
                            CHANGE
                          </button>
                        )}
                      </div>
                    ))}
                    {shortcutProblem && (
                      <p className="problem" role="alert">
                        {shortcutProblem}
                      </p>
                    )}
                    <p className="field-note">
                      Both work in every app, Mimi included.{" "}
                      <button
                        type="button"
                        className="text-button"
                        disabled={changing !== null}
                        onClick={() => void resetShortcuts()}
                      >
                        Use the original shortcuts
                      </button>
                    </p>
                  </div>

                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.chimes}
                      onChange={(event) => void changeSwitch("set_chimes", event.target.checked)}
                    />
                    <span>Chime when I start and stop</span>
                  </label>
                  <p className="field-note">
                    A rising "tu-rún" when Mimi starts listening, a falling one when she stops.
                  </p>

                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.quiet_other_apps}
                      onChange={(event) => void changeSwitch("set_quiet_other_apps", event.target.checked)}
                    />
                    <span>Silence other apps while I talk</span>
                  </label>
                  <p className="field-note">
                    Music and videos fade out while Mimi listens, and come back when you stop.
                  </p>

                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.cleanup}
                      onChange={(event) => void changeSwitch("set_cleanup", event.target.checked)}
                    />
                    <span>Clean up my speech</span>
                  </label>
                  <p className="field-note">
                    Removes "um", "eh" and a hesitant "este", collapses stutters, and follows "new line" and "nueva
                    línea". Counting out loud ("primero… segundo…") makes a numbered list, and "viñeta" or "bullet"
                    before each item makes bullets. Spanish and English only.
                  </p>

                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.animate_mimi}
                      onChange={(event) => void changeSwitch("set_animate_mimi", event.target.checked)}
                    />
                    <span>Animate Mimi</span>
                  </label>
                  <p className="field-note">She only moves while this window is active.</p>

                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.start_with_windows}
                      onChange={(event) => void changeSwitch("set_start_with_windows", event.target.checked)}
                    />
                    <span>Start with Windows</span>
                  </label>
                  <p className="field-note">
                    Mimi waits in the tray from the moment you sign in, ready for{" "}
                    {shortcutLabel(settings.hold_shortcut)}.
                  </p>
                </>
              )}

              {clearCount === null ? (
                <button className="quit" type="button" onClick={() => void askToClear()}>
                  DELETE ALL DICTATIONS
                </button>
              ) : clearCount === 0 ? (
                <p className="field-note" role="status">
                  There are no dictations to delete.{" "}
                  <button type="button" className="text-button" onClick={() => setClearCount(null)}>
                    OK
                  </button>
                </p>
              ) : (
                <div className="confirm-all" role="alertdialog" aria-label="Delete all dictations">
                  <p>
                    Delete all {clearCount} {clearCount === 1 ? "dictation" : "dictations"} from this PC? Snippets stay.
                    This can't be undone.
                  </p>
                  <p className="confirm">
                    <button type="button" className="confirm-delete" onClick={() => void clearHistory()}>
                      DELETE ALL
                    </button>
                    <button type="button" className="confirm-keep" onClick={() => setClearCount(null)}>
                      KEEP
                    </button>
                  </p>
                </div>
              )}

              <button className="quit" type="button" onClick={() => void invoke("quit_app")}>
                QUIT MIMI
              </button>
              <p className="field-note">The X only hides Mimi. This closes her completely.</p>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
