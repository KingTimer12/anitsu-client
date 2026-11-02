import { createRoot, createSignal } from "solid-js";
import type { FileEntry } from "../anitsu";

/** O que está aberto no leitor. `dir` é a pasta dos arquivos; `titlePath`, a chave das preferências. */
export interface ReadTarget {
  dir: string;
  file: FileEntry;
  files: FileEntry[];
  titlePath: string;
  title: string;
}

export const reader = createRoot(() => {
  const [current, setCurrent] = createSignal<ReadTarget | null>(null);
  return {
    current,
    open: (t: ReadTarget) => setCurrent(t),
    close: () => setCurrent(null),
  };
});
