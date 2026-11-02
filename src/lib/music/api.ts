import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "../anitsu/invoke";
import type { Album, FormatInfo, Track } from "./logic";
import type { RepeatMode } from "./store";

export type { FormatInfo };

/** Estado emitido pelo motor em Rust (`music://state`). */
export interface PlayerState {
  album: string | null;
  index: number | null;
  path: string | null;
  playing: boolean;
  position: number;
  duration: number | null;
  /** 0..1 da faixa já baixada. */
  buffered: number;
  format: FormatInfo | null;
  volume: number;
  shuffle: boolean;
  repeat: RepeatMode;
  normalize: boolean;
  error: string | null;
  loading: boolean;
  ended: boolean;
  title: string | null;
  artist: string | null;
}

export interface TrackMeta {
  title: string | null;
  artist: string | null;
  duration: number | null;
  codec: string;
  bits: number | null;
  sample_rate: number;
  lossless: boolean;
  cover: string | null;
}

export type MusicCommand = "pause" | "resume" | "toggle" | "next" | "prev" | "stop";

const queueOf = (album: Album) => album.tracks.map((t) => ({ path: t.path, size: t.size, ext: t.ext }));

/** Comandos do player. O JS nunca toca a sessão nem o áudio: só pede ao Rust. */
export const musicApi = {
  play: (album: Album, index: number, at = 0) => invoke<void>("music_play", { album: album.path, tracks: queueOf(album), index, at }),
  command: (cmd: MusicCommand) => invoke<void>("music_command", { cmd }),
  seek: (secs: number) => invoke<void>("music_seek", { secs }),
  jump: (index: number) => invoke<void>("music_jump", { index }),
  set: (o: Partial<{ volume: number; shuffle: boolean; repeat: RepeatMode; normalize: boolean }>) => invoke<void>("music_set", o),
  meta: (t: Track, wantCover = false) => invoke<TrackMeta>("music_meta", { path: t.path, size: t.size, ext: t.ext, wantCover }),
  onState: (cb: (s: PlayerState) => void): Promise<UnlistenFn> => listen<PlayerState>("music://state", (e) => cb(e.payload)),
  /** Imagem da nuvem (capa, scans) pelo protocolo `nuvemimg://`; o Rust busca com a sessão. */
  imageUrl: (path: string, size: number) => convertFileSrc(`${size}/${path}`, "nuvemimg"),
};
