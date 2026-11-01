import { createRoot, createSignal } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { type PlayerData, playerApi, type Progress, type Settings } from "./api";
import type { TrackPref } from "./logic";

export const DEFAULT_SETTINGS: Settings = {
  autoNext: true,
  countdownSec: 5,
  showSkipIntro: true,
  subLang: "por",
  audioLang: "jpn",
  cacheMib: 150,
};

const SAVE_DEBOUNCE_MS = 5000;

/** Progresso, faixas e configurações do player; carregado uma vez do `player.json`. */
export const playerStore = createRoot(() => {
  const [ready, setReady] = createSignal(false);
  const [data, setData] = createStore<PlayerData>({ progress: {}, tracks: {}, settings: DEFAULT_SETTINGS });
  const pending = new Map<string, ReturnType<typeof setTimeout>>();

  async function reload() {
    try {
      setData(reconcile(await playerApi.data()));
    } catch {
      // Sem estado salvo: segue com o padrão.
    }
    setReady(true);
  }
  void reload();

  const saveProgressNow = (path: string) => {
    clearTimeout(pending.get(path));
    pending.delete(path);
    const p = data.progress[path];
    return p ? playerApi.saveProgress(path, { ...p }).catch(() => {}) : Promise.resolve();
  };

  return {
    ready,
    reload,
    progress: (path: string): Progress | undefined => data.progress[path],
    settings: () => data.settings,
    tracks: (release: string): TrackPref | undefined => data.tracks[release],
    /** Atualiza na hora; grava com debounce por arquivo (ou já, em `flush`). */
    setProgress(path: string, p: Progress) {
      setData("progress", path, p);
      if (!pending.has(path)) pending.set(path, setTimeout(() => void saveProgressNow(path), SAVE_DEBOUNCE_MS));
    },
    setTracks(release: string, pref: TrackPref) {
      setData("tracks", release, pref);
      void playerApi.saveTracks(release, pref).catch(() => {});
    },
    setSettings(s: Settings) {
      setData("settings", s);
      void playerApi.saveSettings(s).catch(() => {});
    },
    async flush() {
      await Promise.all([...pending.keys()].map(saveProgressNow));
    },
  };
});
