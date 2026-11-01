import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "./invoke";

export interface FileEntry {
  name: string;
  is_directory: boolean;
  size: number;
  /** ISO 8601 sem timezone. */
  modified: string;
  /** Ex.: `".mkv"`; `null` para diretórios. */
  extension: string | null;
}

export interface Listing {
  path: string;
  parent: string | null;
  files: FileEntry[];
}

/** `other` = categoria nova que o servidor passou a mandar. */
export type SearchKind = "anime" | "desenho" | "dorama" | "manga" | "ost" | "folder" | "other";

export const searchKindLabel: Record<SearchKind, string> = {
  anime: "Anime",
  desenho: "Desenho",
  dorama: "Dorama",
  manga: "Mangá",
  ost: "OST",
  folder: "Pasta",
  other: "Outro",
};

export interface SearchResult {
  name: string;
  /** Caminho do diretório; serve direto para `nuvem.list`. */
  path: string;
  kind: SearchKind;
}

export interface SearchResponse {
  query: string;
  results: SearchResult[];
}

export interface DownloadProgress {
  id: string;
  path: string;
  downloaded: number;
  total: number | null;
  done: boolean;
}

/** Caminho completo de um item dentro da listagem. */
export const joinPath = (dir: string, name: string) => (dir ? `${dir}/${name}` : name);

/** Pastas do acervo que o app não mostra: `AniGUI (Downloader Oficial)` é de outro aplicativo. */
const HIDDEN = [/^anigui\b/i];
const isHidden = (name: string) => HIDDEN.some((re) => re.test(name.trim()));
const hiddenPath = (path: string) => path.split("/").some(isHidden);

/** API da nuvem.anitsu.moe (onde ficam os animes). */
export const nuvem = {
  /** Lista diretório. `""` = raiz. */
  list: async (path = "") => {
    const listing = await invoke<Listing>("nuvem_list", { path });
    return { ...listing, files: listing.files.filter((f) => !isHidden(f.name)) };
  },

  /** Busca pastas por nome. Resultados são diretórios (`path` vai direto em `list`). */
  search: async (query: string) => {
    const res = await invoke<SearchResponse>("nuvem_search", { query });
    return { ...res, results: res.results.filter((r) => !hiddenPath(r.path)) };
  },

  /**
   * Baixa em stream direto para o disco (Rust). Sem `dest`, vai para Downloads.
   * Resolve com o caminho final do arquivo.
   */
  download: (path: string, opts: { id?: string; dest?: string } = {}) =>
    invoke<string>("nuvem_download", { id: opts.id ?? crypto.randomUUID(), path, dest: opts.dest }),

  onDownloadProgress: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
    listen<DownloadProgress>("nuvem://download-progress", (e) => cb(e.payload)),
};
