import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import type { Direction, ProgressStyle, ReadMode } from "./logic";

export interface ReadProgress {
  page: number;
  total: number;
  read: boolean;
  /** Unix, segundos. */
  updatedAt: number;
  /**
   * Só no resumo de um arquivo com vários capítulos dentro (coletânea):
   * chave e nome do capítulo onde parou. `page`/`total` contam capítulos.
   */
  unit?: string;
  unitName?: string;
}
export interface TitlePrefs {
  direction: Direction;
  mode: ReadMode;
}
export interface ReaderSettings {
  /** Emendar capítulos (scroll infinito / virar direto para o próximo). */
  seam: boolean;
  progress: ProgressStyle;
}
interface ReaderData {
  progress: Record<string, ReadProgress>;
  prefs: Record<string, TitlePrefs>;
  settings: ReaderSettings;
}

const KEY = "higanbana:reader:v1";
export const DEFAULT_PREFS: TitlePrefs = { direction: "rtl", mode: "single" };
export const DEFAULT_READER_SETTINGS: ReaderSettings = { seam: true, progress: "thread" };

function load(): ReaderData {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<ReaderData> | null;
    return {
      progress: raw?.progress ?? {},
      prefs: raw?.prefs ?? {},
      settings: { ...DEFAULT_READER_SETTINGS, ...raw?.settings },
    };
  } catch {
    return { progress: {}, prefs: {}, settings: DEFAULT_READER_SETTINGS };
  }
}

/** Página onde parou, preferências por título e configurações; fica no armazenamento do webview. */
export const readerStore = createRoot(() => {
  const [data, setData] = createStore<ReaderData>(load());
  let timer: ReturnType<typeof setTimeout> | undefined;
  const persist = () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      try {
        localStorage.setItem(KEY, JSON.stringify(data));
      } catch {
        // Sem espaço: segue só em memória.
      }
    }, 500);
  };

  return {
    progress: (path: string): ReadProgress | undefined => data.progress[path],
    prefs: (titlePath: string): TitlePrefs => data.prefs[titlePath] ?? DEFAULT_PREFS,
    settings: () => data.settings,
    /** `key` = caminho do arquivo ou chave do capítulo interno (ver `unitKey`). */
    setProgress(key: string, page: number, total: number) {
      const prev = data.progress[key];
      const read = (prev?.read ?? false) || page >= total - 1;
      setData("progress", key, { page, total, read, updatedAt: Math.floor(Date.now() / 1000) });
      persist();
    },
    /** Resumo de uma coletânea: em qual capítulo interno parou. */
    setSummary(path: string, s: { index: number; count: number; read: boolean; unit: string; unitName: string }) {
      const prev = data.progress[path];
      setData("progress", path, {
        page: s.index,
        total: s.count,
        read: (prev?.read ?? false) || (s.read && s.index >= s.count - 1),
        updatedAt: Math.floor(Date.now() / 1000),
        unit: s.unit,
        unitName: s.unitName,
      });
      persist();
    },
    setPrefs(titlePath: string, p: Partial<TitlePrefs>) {
      setData("prefs", titlePath, { ...(data.prefs[titlePath] ?? DEFAULT_PREFS), ...p });
      persist();
    },
    setSettings(s: Partial<ReaderSettings>) {
      setData("settings", (old) => ({ ...old, ...s }));
      persist();
    },
  };
});
