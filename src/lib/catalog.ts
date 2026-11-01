import { lookupMedia } from "./anilist/api";
import type { MediaSummary } from "./anilist/types";
import { joinPath, nuvem } from "./anitsu";
import { folderTitle } from "./library";

/**
 * Descobre quais animes do AniList existem no acervo, pegando leve na API:
 * - lista a pasta da letra (`Animes/Letra X`) uma vez e compara nomes localmente,
 *   em vez de uma busca por título;
 * - no máximo 2 listagens ao mesmo tempo;
 * - resultado guardado por 12 h (abrir a Home de novo não faz requisição).
 */

const ROOT = "Animes";
const CONCURRENCY = 2;
const TTL_MS = 12 * 60 * 60 * 1000;
const STORE_KEY = "anitsu:catalog:v1";

/** Só letras/números, minúsculo, sem acento. */
const norm = (s: string) =>
  s
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "");

/** Remove sufixos de temporada/subtítulo: "Foo Season 2", "Foo 2nd Season", "Foo: Bar". */
const baseTitle = (s: string) =>
  s
    .replace(/\s*[:：].*$/, "")
    .replace(/\s+(\d+(st|nd|rd|th)\s+season|season\s+\d+|part\s+\d+|cour\s+\d+)\b.*$/i, "")
    .trim();

const titlesOf = (m: MediaSummary) =>
  [m.title.romaji, m.title.english, m.title.userPreferred, ...(m.synonyms ?? [])].filter(
    (t): t is string => !!t && /[a-z]/i.test(t),
  );

/** Pasta de letra onde o título estaria: `Letra B` ou `0-9`. */
function letterFolder(title: string): string | null {
  const c = norm(title)[0];
  if (!c) return null;
  return /[0-9]/.test(c) ? "0-9" : `Letra ${c.toUpperCase()}`;
}

// --- Cache persistente { id → caminho | null } ---
type Stored = { at: number; paths: Record<number, string | null> };

function loadStore(): Stored {
  try {
    const raw = localStorage.getItem(STORE_KEY);
    const s = raw ? (JSON.parse(raw) as Stored) : null;
    if (s && Date.now() - s.at < TTL_MS) return s;
  } catch {
    // Armazenamento indisponível: segue sem cache.
  }
  return { at: Date.now(), paths: {} };
}

const store = loadStore();
function saveStore() {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(store));
  } catch {
    // Ignora: cache é só conveniência.
  }
}

// --- Listagens de letra, uma vez cada ---
const letterMemo = new Map<string, Promise<string[]>>();
function listLetter(folder: string): Promise<string[]> {
  let p = letterMemo.get(folder);
  if (!p) {
    p = nuvem
      .list(joinPath(ROOT, folder))
      .then((l) => l.files.filter((f) => f.is_directory).map((f) => f.name))
      .catch((e) => {
        letterMemo.delete(folder);
        throw e;
      });
    letterMemo.set(folder, p);
  }
  return p;
}

async function resolve(m: MediaSummary): Promise<string | null> {
  const titles = titlesOf(m);
  const exact = new Set(titles.map(norm));
  const bases = new Set(titles.map((t) => norm(baseTitle(t))).filter((b) => b.length >= 3));
  const letters = [...new Set(titles.slice(0, 2).map(letterFolder).filter(Boolean) as string[])];

  const near: string[] = [];
  for (const letter of letters) {
    for (const name of await listLetter(letter)) {
      const n = norm(folderTitle(name));
      const path = joinPath(joinPath(ROOT, letter), name);
      if (exact.has(n)) return path;
      if (bases.has(n)) near.push(path);
    }
  }
  // Nome parecido (ex.: sequência numa pasta com o nome base): confirma pelo AniList
  // (lookup em lote e com cache em disco, não toca na nuvem).
  for (const path of near) {
    const found = await lookupMedia(path.split("/").at(-1)!, "ANIME");
    if (found?.id === m.id) return path;
  }
  // A pasta é da obra base e a temporada nova fica dentro dela: aceita a pasta base.
  return near[0] ?? null;
}

/**
 * Filtra os títulos que existem no acervo. `onFound` é chamado conforme cada um resolve.
 */
export async function filterInNuvem(
  items: MediaSummary[],
  onFound: (m: MediaSummary, path: string, index: number) => void,
): Promise<void> {
  const todo: number[] = [];
  items.forEach((m, i) => {
    if (m.id in store.paths) {
      const path = store.paths[m.id];
      if (path) onFound(m, path, i);
    } else todo.push(i);
  });
  if (!todo.length) return;

  let next = 0;
  const worker = async () => {
    while (next < todo.length) {
      const i = todo[next++];
      try {
        const path = await resolve(items[i]);
        store.paths[items[i].id] = path;
        if (path) onFound(items[i], path, i);
      } catch {
        // Falha de rede: não guarda, tenta de novo na próxima abertura.
      }
    }
  };
  await Promise.all(Array.from({ length: CONCURRENCY }, worker));
  saveStore();
}
