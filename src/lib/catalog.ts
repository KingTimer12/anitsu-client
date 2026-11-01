import { createStore } from "solid-js/store";
import { lookupMedia } from "./anilist/api";
import type { MediaSummary } from "./anilist/types";
import { joinPath, nuvem } from "./anitsu";
import { folderTitle } from "./library";

/**
 * Confere se um anime do AniList está no acervo. Feito sob demanda: o chamador
 * pede só quando o card aparece na tela. Economia de API:
 * - lista a pasta da letra (`Animes/Letra X`) uma vez e compara nomes localmente;
 * - nomes parecidos são confirmados no AniList (lote + cache em disco);
 * - no máximo 2 pedidos em paralelo;
 * - cada título é conferido uma vez; resultado guardado por 12 h.
 */

const ROOT = "Animes";
const CONCURRENCY = 2;
const TTL_MS = 12 * 60 * 60 * 1000;
const STORE_KEY = "higanbana:catalog:v1";

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

const initial = loadStore();
const storedAt = initial.at;

/** Estado reativo: `undefined` = não conferido, `null` = não está no acervo, string = caminho. */
const [state, setState] = createStore<Record<number, string | null>>(initial.paths);

export function nuvemPath(id: number): string | null | undefined {
  return state[id];
}

function save() {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify({ at: storedAt, paths: state }));
  } catch {
    // Ignora: cache é só conveniência.
  }
}

// --- Fila com limite de concorrência ---
let active = 0;
const waiting: (() => void)[] = [];
function limit<T>(task: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      active++;
      task()
        .then(resolve, reject)
        .finally(() => {
          active--;
          waiting.shift()?.();
        });
    };
    if (active < CONCURRENCY) run();
    else waiting.push(run);
  });
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
  // Nome parecido (ex.: sequência numa pasta com o nome base): confirma pelo AniList.
  for (const path of near) {
    const found = await lookupMedia(path.split("/").at(-1)!, "ANIME");
    if (found?.id === m.id) return path;
  }
  // A temporada nova costuma ficar dentro da pasta da obra base.
  return near[0] ?? null;
}

const inflight = new Map<number, Promise<string | null>>();

/**
 * Confere um título no acervo. Chamadas repetidas reaproveitam o resultado.
 * Falha de rede não é guardada: tenta de novo na próxima vez.
 */
export function checkNuvem(m: MediaSummary): Promise<string | null> {
  const known = state[m.id];
  if (known !== undefined) return Promise.resolve(known);
  const pending = inflight.get(m.id);
  if (pending) return pending;

  const p = limit(() => resolve(m))
    .then(
      (path) => {
        setState(m.id, path);
        save();
        return path;
      },
      () => null,
    )
    .finally(() => inflight.delete(m.id));
  inflight.set(m.id, p);
  return p;
}
