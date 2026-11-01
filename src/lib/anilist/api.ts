import { invoke } from "../anitsu/invoke";
import type { MediaDetail, MediaSummary, MediaType } from "./types";

export interface LookupItem {
  /** Nome da pasta na nuvem. O Rust limpa tags e extrai o ano. */
  name: string;
  type: MediaType;
}

export const anilist = {
  lookup: (items: LookupItem[]) => invoke<(MediaSummary | null)[]>("anilist_lookup", { items }),
  media: (id: number) => invoke<MediaDetail>("anilist_media", { id }),
  trending: () => invoke<MediaSummary[]>("anilist_trending"),
};

const pending = new Map<string, { item: LookupItem; resolve: (m: MediaSummary | null) => void }[]>();
const memo = new Map<string, Promise<MediaSummary | null>>();
let timer: ReturnType<typeof setTimeout> | undefined;

const BATCH_DELAY_MS = 60;
const MAX_PER_CALL = 40;

/**
 * Casa uma pasta com o AniList. Chamadas próximas no tempo viram uma única
 * requisição em lote (o AniList limita a ~30 req/min). Resultado memorizado.
 */
export function lookupMedia(name: string, type: MediaType): Promise<MediaSummary | null> {
  const key = `${type}|${name}`;
  const cached = memo.get(key);
  if (cached) return cached;

  const promise = new Promise<MediaSummary | null>((resolve) => {
    const waiters = pending.get(key) ?? [];
    waiters.push({ item: { name, type }, resolve });
    pending.set(key, waiters);
    timer ??= setTimeout(flush, BATCH_DELAY_MS);
  });
  memo.set(key, promise);
  return promise;
}

async function flush() {
  timer = undefined;
  const entries = [...pending.entries()].slice(0, MAX_PER_CALL);
  for (const [key] of entries) pending.delete(key);
  if (pending.size) timer = setTimeout(flush, BATCH_DELAY_MS);

  try {
    const results = await anilist.lookup(entries.map(([, w]) => w[0].item));
    entries.forEach(([, waiters], i) => waiters.forEach((w) => w.resolve(results[i] ?? null)));
  } catch {
    // Falha de rede/limite: libera para tentar de novo depois.
    for (const [key, waiters] of entries) {
      memo.delete(key);
      waiters.forEach((w) => w.resolve(null));
    }
  }
}
