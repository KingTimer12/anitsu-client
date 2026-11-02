import type { Album, Track } from "./logic";

/**
 * Metadados das faixas, **um pedido por vez** (a nuvem responde 429 a pedidos seguidos).
 * Pedir o mesmo álbum de novo não duplica nada; trocar de álbum para o anterior
 * depois do pedido em andamento.
 */
export function createMetaQueue<T>(fetch: (t: Track) => Promise<T>, store: { has(path: string): boolean; set(path: string, v: T): void }) {
  let token = 0;
  let running: string | null = null;
  let current: Promise<void> = Promise.resolve();

  return {
    load(a: Album): Promise<void> {
      if (running === a.path) return current;
      const my = ++token;
      running = a.path;
      const prev = current;
      current = (async () => {
        await prev;
        for (const t of a.tracks) {
          if (my !== token) return;
          if (store.has(t.path)) continue;
          try {
            const v = await fetch(t);
            if (my === token) store.set(t.path, v);
          } catch {
            // Sem metadados: a faixa segue com o nome do arquivo.
          }
        }
      })().finally(() => {
        if (my === token) running = null;
      });
      return current;
    },
  };
}
