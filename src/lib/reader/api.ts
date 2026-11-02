import { convertFileSrc } from "@tauri-apps/api/core";
import { invoke } from "../anitsu/invoke";
import type { OpenResult } from "./logic";

/** Comandos do leitor. A sessão nunca passa por aqui: o Rust busca os bytes. */
export const readerApi = {
  /** Lê só o índice do `.cbz` (ou do `inner` guardado dentro dele) e devolve o conteúdo. */
  open: (path: string, size: number, inner: string | null) => invoke<OpenResult>("reader_open", { path, size, inner }),
  /** URL da imagem pelo protocolo `manga://` (no Windows, `http://manga.localhost/`). */
  pageUrl: (path: string, size: number, inner: string | null, index: number) =>
    convertFileSrc(`${size}/${index}/${encodeURIComponent(inner ?? "")}/${path}`, "manga"),
};
