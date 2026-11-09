// A hidden end-to-end test of the screens-to-engine connection. It runs only when
// started with test settings, shows no window, and uses WAV files instead of a microphone.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Dictation, EngineStatus } from "./App";

export type SelfTestPlan = { clips: string[] };

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function expectRejection(action: () => Promise<unknown>) {
  try {
    await action();
    return { rejected: false, message: "" };
  } catch (error) {
    return { rejected: true, message: String(error) };
  }
}

export async function runSelfTest(plan: SelfTestPlan) {
  let levels = 0;
  let readyEvents = 0;
  const removeLevel = await listen("mic-level", () => {
    levels += 1;
  });
  const removeReady = await listen<{ phase: string }>("dictation-stage", ({ payload }) => {
    if (payload.phase === "listening") readyEvents += 1;
  });

  const report: Record<string, unknown> = {};
  report.stopWithoutStart = await expectRejection(() => invoke("stop_recording"));
  report.missingMicrophone = await expectRejection(() =>
    invoke("start_recording", { device: "A microphone that does not exist" }),
  );
  report.microphonesListed = await invoke("list_microphones");

  const dictations = [];
  const switches: unknown[] = [];
  for (const clip of plan.clips) {
    // An entry can simply wait, to test what happens after Mimi sits unused.
    if (clip.startsWith("wait|")) {
      await wait(Number(clip.split("|")[1]) * 1000);
      continue;
    }
    // An entry can delete every dictation, as the Settings button does.
    if (clip === "clear-check") {
      const before = await invoke<number>("history_count");
      const removed = await invoke<number>("history_clear");
      const after = await invoke<number>("history_count");
      const listed = await invoke<{ id: number }[]>("history_recent", { beforeId: null });
      const snippets = await invoke<unknown[]>("list_snippets");
      report.clearCheck = { before, removed, after, listed: listed.length, snippetsLeft: snippets.length };
      continue;
    }
    // An entry can check history search and delete with the dictations made so far.
    if (clip.startsWith("history-check|")) {
      const word = clip.split("|")[1];
      const found = await invoke<{ id: number }[]>("history_search", { query: word });
      const accentless = await invoke<{ id: number }[]>("history_search", { query: word.toUpperCase() });
      const before = await invoke<{ id: number }[]>("history_recent", { beforeId: null });
      const deleted = found[0] ? await invoke<boolean>("history_delete", { id: found[0].id }) : false;
      const afterSearch = await invoke<{ id: number }[]>("history_search", { query: word });
      const after = await invoke<{ id: number }[]>("history_recent", { beforeId: null });
      report.historyCheck = {
        word,
        found: found.length,
        foundInCapitals: accentless.length,
        deleted,
        foundAfterDelete: afterSearch.length,
        listBefore: before.length,
        listAfter: after.length,
      };
      continue;
    }
    // An entry can add a snippet instead of being a clip.
    if (clip.startsWith("snippet|")) {
      const [, phrase, text] = clip.split("|");
      await invoke("save_snippet", { id: null, phrase, text: text.replace("\\n", "\n") });
      continue;
    }
    // Each clip can also name a model and language to switch to first.
    const [path, seconds, model, language] = clip.split("|");
    if (model) {
      const started = performance.now();
      await invoke("change_settings", { model, language: language || null });
      let status = await invoke<EngineStatus>("engine_status");
      while (status.state === "loading") {
        await wait(200);
        status = await invoke<EngineStatus>("engine_status");
      }
      switches.push({ model, language, seconds: (performance.now() - started) / 1000, status });
    }
    levels = 0;
    readyEvents = 0;
    await invoke("start_recording", { device: `test-wav:${path}` });
    await wait(Number(seconds) * 1000 + 300);
    const doubleStart = await expectRejection(() =>
      invoke("start_recording", { device: `test-wav:${path}` }),
    );
    const dictation = await invoke<Dictation>("stop_recording");
    dictations.push({ clip: path, dictation, levels, readyEvents, doubleStart });
  }
  report.dictations = dictations;
  report.switches = switches;
  report.settings = await invoke("get_settings");
  report.models = await invoke("list_models");
  report.engine = await invoke("engine_status");
  report.history = await invoke("history_recent", { beforeId: null });
  report.snippets = await invoke("list_snippets");
  const animateOff = await invoke<{ animate_mimi: boolean }>("set_animate_mimi", { enabled: false });
  const animateOn = await invoke<{ animate_mimi: boolean }>("set_animate_mimi", { enabled: true });
  let hideAllowed = true;
  try {
    await getCurrentWindow().hide();
  } catch {
    hideAllowed = false;
  }
  report.screen = {
    oxaniumLoaded: (await document.fonts.load('700 46px "Oxanium Variable"')).length > 0,
    manropeLoaded: (await document.fonts.load('500 14px "Manrope Variable"')).length > 0,
    mimiCanvasWidth: document.querySelector("canvas")?.width ?? 0,
    animateSwitchWorks: animateOff.animate_mimi === false && animateOn.animate_mimi === true,
    hideAllowed,
  };

  removeLevel();
  removeReady();
  await invoke("self_test_finish", { report });
}
