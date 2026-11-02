import { createEffect, createRoot, createSignal, on, untrack } from "solid-js";
import { createStore } from "solid-js/store";
import { player } from "../player/session";
import { musicApi, type PlayerState, type TrackMeta } from "./api";
import type { Album } from "./logic";
import { createMetaQueue } from "./metaQueue";
import { musicStore } from "./store";

/** Salvar onde parou a cada tantos ms enquanto toca. */
const SAVE_EVERY_MS = 5000;

/** Estado global do player de músicas (a barra e o encarte leem daqui). */
export const music = createRoot(() => {
  const [state, setState] = createSignal<PlayerState | null>(null);
  const [album, setAlbum] = createSignal<Album | null>(null);
  const [metas, setMetas] = createStore<Record<string, TrackMeta>>({});
  let lastSave = 0;
  const metaQueue = createMetaQueue((t) => musicApi.meta(t), {
    has: (p) => !!metas[p],
    set: (p, m) => setMetas(p, m),
  });

  const saveProgress = (s: PlayerState) => {
    const a = album();
    if (!a || s.album !== a.path || s.index === null) return;
    lastSave = Date.now();
    // Álbum acabou: a próxima vez começa do início.
    if (s.ended) musicStore.setProgress(a.path, 0, 0);
    else musicStore.setProgress(a.path, s.index, s.position);
  };

  void musicApi.onState((s) => {
    const prev = state();
    setState(s);
    const changed = !prev || prev.index !== s.index || prev.playing !== s.playing || prev.ended !== s.ended;
    if (changed || (s.playing && Date.now() - lastSave > SAVE_EVERY_MS)) saveProgress(s);
    if (prev && (prev.volume !== s.volume || prev.shuffle !== s.shuffle || prev.repeat !== s.repeat || prev.normalize !== s.normalize)) {
      musicStore.setPrefs({ volume: s.volume, shuffle: s.shuffle, repeat: s.repeat, normalize: s.normalize });
    }
  });

  // Preferências salvas valem desde o início.
  void musicApi.set(musicStore.prefs()).catch(() => {});

  // Abrir um vídeo pausa a música (o leitor de mangá não).
  createEffect(
    on(player.current, (c) => {
      if (c && state()?.playing) void musicApi.command("pause");
    }),
  );

  /** Tags e duração das faixas, uma por vez (a nuvem limita pedidos seguidos). */
  const loadMeta = (a: Album) => metaQueue.load(a);

  return {
    state,
    album,
    meta: (path: string): TrackMeta | undefined => metas[path],
    loadMeta,
    open(a: Album, index: number, at = 0) {
      setAlbum(a);
      void musicApi.play(a, index, at);
    },
    /** Encarte aberto sem tocar (para "Continuar" e metadados). */
    show(a: Album) {
      untrack(() => {
        if (state()?.album === a.path && album()?.path !== a.path) setAlbum(a);
        void loadMeta(a);
      });
    },
    isCurrent: (a: Album) => state()?.album === a.path,
  };
});
