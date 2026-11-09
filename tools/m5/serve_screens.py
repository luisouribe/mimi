"""Serves the built screens on port 1421 with simulated Mimi data, for checks with no real app.

Add ?stage=listening&level=0.6&view=settings&empty=1&animate=0&clear=1 to the address to pick a state.
"""
import http.server
import os
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve()

MOCK = r"""<script>
(() => {
  const query = new URLSearchParams(location.search);
  const now = Date.now();
  const sample = [
    { id: 4, text: "Llego en veinte minutos. Guárdame un sitio, por favor.", app_name: "WhatsApp", created_at_ms: now - 60000, word_count: 8, speech_seconds: 3, pasted: true },
    { id: 3, text: "Hola Juan,\nGracias por tu mensaje.\n\nNos vemos mañana.", app_name: "Gmail", created_at_ms: now - 600000, word_count: 8, speech_seconds: 4, pasted: true },
    { id: 2, text: "Give me five title ideas for a short video about old Tokyo train stations.", app_name: "ChatGPT", created_at_ms: now - 3600000, word_count: 14, speech_seconds: 5, pasted: true },
    { id: 1, text: "今日は予約がたくさんあって疲れました。", app_name: "Notepad", created_at_ms: now - 90000000, word_count: 1, speech_seconds: 3, pasted: true },
  ];
  let settings = { model: query.get("model") || "parakeet", language: query.get("language"), whisper_cpu_only: false, cleanup: true, animate_mimi: query.get("animate") !== "0", start_with_windows: query.get("startup") !== "0", chimes: true, quiet_other_apps: true, hold_shortcut: { ctrl: true, alt: false, shift: false, key: 32, key_name: "Space" }, tap_shortcut: { ctrl: true, alt: false, shift: true, key: 32, key_name: "Space" }, appearance: query.get("theme") || "light" };
  let cleared = false;
  const callbacks = new Map();
  const handlers = new Map();
  let next = 1;
  const answers = {
    self_test_plan: () => null,
    list_microphones: () => [{ name: "Headset Microphone", is_default: true }, { name: "USB Microphone", is_default: false }],
    set_microphone: () => null,
    history_recent: (args) => {
      if (query.get("empty")) return [];
      if (!query.get("many")) return sample;
      const top = args.beforeId ?? 251;
      const page = [];
      for (let id = top - 1; id >= Math.max(1, top - 100); id--) {
        page.push({ id, text: "Dictado simulado numero " + id, app_name: "Chrome", created_at_ms: now - id * 60000, word_count: 4, speech_seconds: 2, pasted: true });
      }
      return page;
    },
    list_models: () => [
      { id: "parakeet", name: "Parakeet V3", description: "25 European languages, Spanish and English included. Fastest, and no graphics card needed.", installed: true },
      { id: "whisper-turbo", name: "Whisper Turbo", description: "100 languages, Japanese included. The most accurate Whisper.", installed: true },
      { id: "whisper-small", name: "Whisper Small", description: "99 languages, Japanese included. Lighter, but less accurate.", installed: true },
    ],
    get_settings: () => settings,
    engine_status: () => ({ state: "ready", model: settings.model, load_seconds: 1, uses_graphics_card: false, problem: null }),
    change_settings: (args) => (settings = { ...settings, model: args.model, language: args.language }),
    set_cleanup: (args) => (settings = { ...settings, cleanup: args.enabled }),
    set_animate_mimi: (args) => (settings = { ...settings, animate_mimi: args.enabled }),
    set_start_with_windows: (args) => (settings = { ...settings, start_with_windows: args.enabled }),
    set_chimes: (args) => (settings = { ...settings, chimes: args.enabled }),
    set_appearance: (args) => (settings = { ...settings, appearance: args.appearance }),
    get_appearance: () => settings.appearance,
    change_shortcut: () => new Promise((resolve) => setTimeout(() => resolve(null), 600000)),
    cancel_shortcut_change: () => null,
    note_from_screen: () => null,
    shortcut_from_screen: (args) => { window.__shortcuts = [...(window.__shortcuts || []), args.action]; return null; },
    shortcut_captured_from_screen: (args) => { window.__captured = args; return null; },
    reset_shortcuts: () => settings,
    set_quiet_other_apps: (args) => (settings = { ...settings, quiet_other_apps: args.enabled }),
    history_count: () => (query.get("empty") || cleared ? 0 : 32),
    history_clear: () => { cleared = true; return 32; },
    window_ready: () => { window.__windowReady = true; return null; },
    copy_text: () => null,
    history_search: (args) => sample.filter((entry) => entry.text.toLowerCase().includes(args.query.toLowerCase())),
    history_delete: () => true,
    list_snippets: () => (query.get("empty") ? [] : [
      { id: 1, phrase: "mi dirección", text: "Calle Luna 5, Colonia Centro,\nCiudad de México" },
      { id: 2, phrase: "my bio", text: "I am a video producer and editor based in Mexico City, working on short films, documentaries and social campaigns for brands across Latin America and beyond." },
      { id: 3, phrase: "respuesta estándar", text: "Gracias por escribir. Te contesto pronto." },
    ]),
    save_snippet: (args) => ({ id: 9, ...args }),
    delete_snippet: () => true,
    quit_app: () => { window.__quit = true; return null; },
    "plugin:event|listen": (args) => { const id = next++; handlers.set(id, { event: args.event, handler: args.handler }); return id; },
    "plugin:event|unlisten": () => null,
    "plugin:window|is_focused": () => true,
    "plugin:window|minimize": () => { window.__minimized = true; return null; },
    "plugin:window|hide": () => { window.__hidden = true; return null; },
    "plugin:window|is_maximized": () => query.get("max") === "1",
    "plugin:window|toggle_maximize": () => { window.__maximized = !window.__maximized; return null; },
  };
  window.__calls = [];
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback(callback) { const id = next++; callbacks.set(id, callback); return id; },
    async invoke(command, args) {
      window.__calls.push(command);
      if (command in answers) return answers[command](args || {});
      throw new Error("unexpected command " + command);
    },
  };
  window.__emit = (event, payload) => {
    for (const { event: name, handler } of handlers.values()) {
      if (name === event) callbacks.get(handler)?.({ event, id: 0, payload });
    }
  };
  // The indicator floats over other apps, so its pictures get a backdrop to stand in for one.
  if (query.get("backdrop")) {
    document.documentElement.style.background = "#" + query.get("backdrop");
  }
  // Opens every language list, or shows a shortcut waiting for new keys, for screenshots.
  if (query.get("languages")) {
    setTimeout(() => document.querySelectorAll(".languages details").forEach((item) => (item.open = true)), 600);
  }
  if (query.get("scrollto")) {
    setTimeout(() => {
      const target = document.querySelector(query.get("scrollto"));
      const list = target?.closest(".settings-list");
      if (target && list) list.scrollTop = target.offsetTop - list.offsetTop;
    }, 900);
  }
  if (query.get("changing")) {
    setTimeout(() => document.querySelectorAll(".shortcut-row .small-button")[query.get("changing") === "tap" ? 1 : 0]?.click(), 600);
  }
  const stage = query.get("stage");
  if (stage) {
    setTimeout(() => {
      window.__emit("dictation-stage", { phase: stage });
      const level = Number(query.get("level") || 0);
      if (level) setInterval(() => window.__emit("mic-level", level), 60);
    }, 400);
  }
  if (query.get("search")) {
    setTimeout(() => {
      const input = document.querySelector(".search input");
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, query.get("search"));
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, 600);
  }
  if (query.get("confirm")) {
    setTimeout(() => document.querySelector('[aria-label="Delete this dictation"]')?.click(), 600);
  }
  if (query.get("clear")) {
    setTimeout(() => [...document.querySelectorAll(".settings button")].find((b) => b.textContent.includes("DELETE ALL DICTATIONS"))?.click(), 900);
  }
  if (query.get("clear") || query.get("bottom")) {
    setTimeout(() => document.querySelectorAll(".settings-list, .settings").forEach((list) => (list.scrollTop = 100000)), 1200);
  }
  if (query.get("edit")) {
    setTimeout(() => document.querySelector(".panel .talk")?.click(), 600);
  }
  if (query.get("view")) {
    setTimeout(() => [...document.querySelectorAll(".nav button")].find((b) => b.textContent.trim().toLowerCase() === query.get("view"))?.click(), 300);
  }
})();
</script>"""


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def do_GET(self):
        path = self.path.split("?")[0]
        if path in ("/", "/index.html", "/indicator.html"):
            page = "indicator.html" if path == "/indicator.html" else "index.html"
            body = (ROOT / page).read_text(encoding="utf-8").replace("<head>", "<head>" + MOCK, 1).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        super().do_GET()

    def log_message(self, *args):
        pass


os.chdir(ROOT)
http.server.ThreadingHTTPServer(("127.0.0.1", 1421), Handler).serve_forever()
