import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";

/** Onde parou em cada álbum e preferências do player; fica no armazenamento do webview. */

export type RepeatMode = "off" | "all" | "one";

export interface AlbumProgress {
  /** Índice da faixa no álbum. */
  index: number;
  position: number;
  /** Unix, segundos. */
  updatedAt: number;
}

export interface MusicPrefs {
  /** 0..1 (a curva perceptiva é aplicada no Rust). */
  volume: number;
  shuffle: boolean;
  repeat: RepeatMode;
  /** Volume igualado entre álbuns. */
  normalize: boolean;
}

interface MusicData {
  progress: Record<string, AlbumProgress>;
  prefs: MusicPrefs;
}

const KEY = "higanbana:music:v1";
export const DEFAULT_MUSIC_PREFS: MusicPrefs = { volume: 0.8, shuffle: false, repeat: "all", normalize: true };

interface Storage {
  getItem(k: string): string | null;
  setItem(k: string, v: string): void;
}

function load(storage: Storage): MusicData {
  try {
    const raw = JSON.parse(storage.getItem(KEY) ?? "null") as Partial<MusicData> | null;
    return { progress: raw?.progress ?? {}, prefs: { ...DEFAULT_MUSIC_PREFS, ...raw?.prefs } };
  } catch {
    return { progress: {}, prefs: { ...DEFAULT_MUSIC_PREFS } };
  }
}

/** `debounceMs` 0 = grava logo (testes). */
export function createMusicStore(storage: Storage, debounceMs = 500) {
  const [data, setData] = createStore<MusicData>(load(storage));
  let timer: ReturnType<typeof setTimeout> | undefined;
  const write = () => {
    try {
      storage.setItem(KEY, JSON.stringify(data));
    } catch {
      // Sem espaço ou bloqueado: segue só em memória.
    }
  };
  const persist = () => {
    clearTimeout(timer);
    if (debounceMs <= 0) write();
    else timer = setTimeout(write, debounceMs);
  };

  return {
    progress: (albumPath: string): AlbumProgress | undefined => data.progress[albumPath],
    prefs: (): MusicPrefs => ({ ...data.prefs }),
    setProgress(albumPath: string, index: number, position: number) {
      setData("progress", albumPath, { index, position, updatedAt: Math.floor(Date.now() / 1000) });
      persist();
    },
    setPrefs(p: Partial<MusicPrefs>) {
      setData("prefs", (old) => ({ ...old, ...p }));
      persist();
    },
  };
}

const noStorage: Storage = { getItem: () => null, setItem: () => {} };

export const musicStore = createRoot(() => createMusicStore(typeof localStorage === "undefined" ? noStorage : localStorage));
