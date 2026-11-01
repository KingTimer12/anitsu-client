import { createRoot, createSignal } from "solid-js";
import type { FileEntry } from "../anitsu";

/** O que está tocando. `dir` é a pasta do release (chave das faixas lembradas). */
export interface PlayTarget {
  dir: string;
  file: FileEntry;
  files: FileEntry[];
  titlePath: string;
  malId: number | null;
  /** Seção `Animes/` (consulta o AniSkip); doramas e desenhos não. */
  isAnime: boolean;
  animeTitle: string;
}

export const player = createRoot(() => {
  const [current, setCurrent] = createSignal<PlayTarget | null>(null);
  return {
    current,
    open: (t: PlayTarget) => setCurrent(t),
    close: () => setCurrent(null),
    switchTo: (file: FileEntry) => setCurrent((c) => (c ? { ...c, file } : c)),
  };
});
