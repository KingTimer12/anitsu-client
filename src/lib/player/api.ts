import { invoke } from "../anitsu/invoke";
import type { SkipTimes, TrackPref } from "./logic";

export interface Progress {
  position: number;
  duration: number;
  watched: boolean;
  /** Unix, segundos. */
  updatedAt: number;
}

export interface Settings {
  autoNext: boolean;
  countdownSec: number;
  showSkipIntro: boolean;
  subLang: string;
  audioLang: string;
  cacheMib: number;
}

export interface PlayerData {
  progress: Record<string, Progress>;
  tracks: Record<string, TrackPref>;
  settings: Settings;
}

/** Comandos `player_*` do Rust. Sessão e cookie nunca passam por aqui. */
export const playerApi = {
  /** Arquivo local já baixado ou URL do proxy de stream. */
  streamUrl: (path: string, size: number) => invoke<string>("player_stream_url", { path, size }),
  data: () => invoke<PlayerData>("player_data"),
  saveProgress: (path: string, progress: Progress) => invoke<void>("player_save_progress", { path, progress }),
  saveTracks: (release: string, pref: TrackPref) => invoke<void>("player_save_tracks", { release, pref }),
  saveSettings: (settings: Settings) => invoke<void>("player_save_settings", { settings }),
  aniskip: (malId: number, episode: number, duration: number) =>
    invoke<SkipTimes>("player_aniskip", { malId, episode, duration }),
};
