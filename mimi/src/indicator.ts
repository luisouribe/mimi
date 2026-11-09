// The small indicator that floats above other apps while dictating.
import "@fontsource-variable/oxanium";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Appearance = "light" | "dark" | "system";

/** Matches the main window: light, or the plum dark mode. */
function applyAppearance(appearance: Appearance) {
  const dark = appearance === "dark" || (appearance === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

try {
  applyAppearance((localStorage.getItem("mimi-appearance") as Appearance | null) ?? "light");
} catch {
  // The saved setting below still arrives.
}
invoke<Appearance>("get_appearance").then(applyAppearance).catch(() => {});
void listen<Appearance>("appearance-changed", ({ payload }) => applyAppearance(payload));

type Stage =
  | { phase: "idle" | "opening" | "listening" | "working" }
  | { phase: "notice"; message: string };

const BAR_COUNT = 14;
// Each bar has its own shape, so the row looks like a voice rather than one block moving.
const BAR_SHAPE = [0.45, 0.7, 1, 0.55, 0.9, 0.35, 0.75, 1, 0.5, 0.85, 0.6, 1, 0.4, 0.7];
const FRAME_MS = 1000 / 30;

const barsBox = document.getElementById("bars")!;
const spinner = document.getElementById("spinner")!;
const label = document.getElementById("label")!;
const clock = document.getElementById("clock")!;

const bars: HTMLElement[] = [];
for (let index = 0; index < BAR_COUNT; index += 1) {
  const bar = document.createElement("span");
  barsBox.append(bar);
  bars.push(bar);
}

let frame = 0;
let lastDraw = 0;
let target = 0;
let level = 0;
let listeningSince = 0;
let clockTimer = 0;

function draw(now: number) {
  frame = requestAnimationFrame(draw);
  if (now - lastDraw < FRAME_MS) return;
  lastDraw = now;
  level += (target - level) * 0.35;
  bars.forEach((bar, index) => {
    const wobble = 0.65 + 0.35 * Math.sin(now / 90 + index * 1.7);
    const height = Math.max(0.1, Math.min(1, 0.1 + level * BAR_SHAPE[index] * wobble));
    bar.style.transform = `scaleY(${height.toFixed(3)})`;
  });
}

function stopMotion() {
  cancelAnimationFrame(frame);
  frame = 0;
  clearInterval(clockTimer);
  clockTimer = 0;
}

function showClock() {
  const seconds = Math.floor((performance.now() - listeningSince) / 1000);
  clock.textContent = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

function show(stage: Stage) {
  stopMotion();
  const listening = stage.phase === "listening";
  const opening = stage.phase === "opening";
  barsBox.hidden = !(listening || opening);
  spinner.toggleAttribute("hidden", stage.phase !== "working");
  clock.hidden = !listening;

  if (stage.phase === "notice") {
    label.textContent = stage.message;
  } else if (opening) {
    label.textContent = "GET READY";
    target = 0;
  } else if (listening) {
    label.textContent = "LISTENING";
    listeningSince = performance.now();
    showClock();
    clockTimer = window.setInterval(showClock, 500);
    frame = requestAnimationFrame(draw);
  } else if (stage.phase === "working") {
    label.textContent = "TYPING";
  }
}

void listen<Stage>("dictation-stage", ({ payload }) => show(payload));
void listen<number>("mic-level", ({ payload }) => {
  target = Math.min(1, Math.sqrt(payload) * 2.6);
});
