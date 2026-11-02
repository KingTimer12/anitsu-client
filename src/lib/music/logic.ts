import { type FileEntry, joinPath } from "../anitsu/nuvem";

/** Lógica pura do player de músicas (sem Tauri): testada em `logic.test.ts`. */

/** O que o decodificador (symphonia 0.5) toca. APE, CUE, WavPack e Opus ficam só para baixar. */
export const AUDIO_EXT = new Set([".flac", ".mp3", ".wav", ".ogg", ".m4a"]);
const IMAGE_EXT = new Set([".jpg", ".jpeg", ".png", ".webp", ".gif"]);

const extOf = (name: string) => (name.includes(".") ? name.slice(name.lastIndexOf(".")).toLowerCase() : "");
const baseOf = (name: string) => name.replace(/\.[a-z0-9]{2,4}$/i, "");
const natural = (a: string, b: string) => a.localeCompare(b, "pt-BR", { numeric: true, sensitivity: "base" });

export const isAudio = (f: FileEntry) => !f.is_directory && AUDIO_EXT.has(extOf(f.name));
const isImage = (f: FileEntry) => !f.is_directory && IMAGE_EXT.has(extOf(f.name));

export interface TrackName {
  disc: number | null;
  number: number | null;
  title: string;
}

/** `1-03 - Título.flac`, `01 - Título`, `01. Título`, `Track 3`. Datas (`1984-01-25 …`) não são número. */
export function parseTrack(fileName: string): TrackName {
  const base = baseOf(fileName).trim();
  const m = base.match(/^(?:(\d{1,2})-)?(\d{1,3})(?:\s*-\s+|\.\s*|\s+)(.+)$/);
  if (m) return { disc: m[1] ? Number(m[1]) : null, number: Number(m[2]), title: m[3].trim() };
  const t = base.match(/^track\s*(\d{1,3})$/i);
  return { disc: null, number: t ? Number(t[1]) : null, title: base };
}

/** Código do disco entre parênteses ou colchetes: `(SVWC-7767)`. */
export function catalogCode(folder: string): string | null {
  const all = [...folder.matchAll(/[[(]([A-Z]{2,6}-\d{3,6}[A-Z]?)[\])]/g)];
  return all.length ? all[all.length - 1][1] : null;
}

/** `2011.05.25 - …` ou `[2008.11.26]` → ano. */
export function albumYear(folder: string): string | null {
  const m = folder.match(/(?:^|[[(\s])((?:19|20)\d\d)(?=[.\-\])\s]|$)/);
  return m ? m[1] : null;
}

/** Nome do álbum sem data na frente nem código/tags no fim. */
export function albumTitle(folder: string): string {
  const t = folder
    .replace(/^\[album\]\s*/i, "")
    .replace(/^\d{4}(?:[.-]\d{2}){0,2}\s*-\s*/, "")
    .replace(/\s*\[[^\]]*\]/g, "")
    .replace(/\s*\([A-Z]{2,6}-\d{3,6}[A-Z]?\)/g, "")
    // "_" separa palavras em nomes como "-hack_Quantum_Main_Theme".
    .replace(/_/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return t || folder;
}

/** `Disc 1`, `CD2`, `Disco 3` → número do disco. */
export function isDiscDir(name: string): number | null {
  const m = name.trim().match(/^(?:disc|disk|disco|cd)\s*[-_.]?\s*(\d{1,2})$/i);
  return m ? Number(m[1]) : null;
}

/** Vídeos e quadrinhos: pasta com eles é de episódios/capítulos, não um álbum. */
const NOT_ALBUM_EXT = new Set([".mkv", ".mp4", ".avi", ".webm", ".m4v", ".mov", ".wmv", ".ts", ".cbz", ".cbr"]);
const hasNonAlbum = (files: FileEntry[]) => files.some((f) => !f.is_directory && NOT_ALBUM_EXT.has(extOf(f.name)));

/**
 * A pasta vira encarte: tem faixas tocáveis (ou só discos em subpastas) e nenhum
 * vídeo/quadrinho. `discFiles` = conteúdo já listado das subpastas de disco.
 */
export function isAlbumFolder(files: FileEntry[], discFiles: FileEntry[][] = []): boolean {
  if (hasNonAlbum(files) || discFiles.some(hasNonAlbum)) return false;
  const discs = files.filter((f) => f.is_directory && isDiscDir(f.name) !== null).length;
  return files.some(isAudio) || discs >= 2;
}

const NAMED_COVER = /^(cover|front|folder)\b/i;

/** Capa: nome "cover/front/folder" (raiz, depois subpastas) > 1ª imagem da raiz > 1ª imagem de subpasta. */
export function pickCover(dir: string, files: FileEntry[], sub: { dir: string; files: FileEntry[] }[] = []): string | null {
  const groups = [{ dir, files }, ...[...sub].sort((a, b) => natural(a.dir, b.dir))].map((g) => ({
    dir: g.dir,
    images: g.files.filter(isImage).sort((a, b) => natural(a.name, b.name)),
  }));
  for (const g of groups) {
    const named = g.images.find((f) => NAMED_COVER.test(baseOf(f.name)));
    if (named) return joinPath(g.dir, named.name);
  }
  for (const g of groups) if (g.images.length) return joinPath(g.dir, g.images[0].name);
  return null;
}

export interface Track {
  path: string;
  size: number;
  /** Nome do arquivo. */
  name: string;
  disc: number;
  number: number | null;
  title: string;
  ext: string;
}

export interface Album {
  path: string;
  title: string;
  catalog: string | null;
  year: string | null;
  tracks: Track[];
  discs: number[];
  coverPath: string | null;
  /** O que não é faixa (scans, `.cue`, `.ape`, subpastas), para a aba "Arquivos". */
  others: FileEntry[];
  /** Subpastas de disco (`CD1`, `Disc 2`) já incorporadas na fila. */
  discDirs: string[];
}

/** Álbum a partir da listagem (e das subpastas de disco já listadas). `null` se nada for tocável. */
export function buildAlbum(path: string, files: FileEntry[], discs: { n: number; dir: string; files: FileEntry[] }[] = []): Album | null {
  const sortedDiscs = [...discs].sort((a, b) => a.n - b.n);
  const toTrack = (dir: string, f: FileEntry, discDefault: number | null): Track => {
    const t = parseTrack(f.name);
    return {
      path: joinPath(dir, f.name),
      size: f.size,
      name: f.name,
      disc: discDefault ?? t.disc ?? 1,
      number: t.number,
      title: t.title,
      ext: extOf(f.name),
    };
  };
  const tracks = [
    ...files.filter(isAudio).map((f) => toTrack(path, f, null)),
    ...sortedDiscs.flatMap((g) => g.files.filter(isAudio).map((f) => toTrack(g.dir, f, g.n))),
  ].sort((a, b) => a.disc - b.disc || (a.number ?? Infinity) - (b.number ?? Infinity) || natural(a.name, b.name));
  if (!tracks.length) return null;

  const discDirNames = new Set(sortedDiscs.map((g) => g.dir.split("/").at(-1)));
  const others = files.filter((f) => !isAudio(f) && !(f.is_directory && discDirNames.has(f.name))).sort((a, b) => natural(a.name, b.name));
  const folder = path.split("/").filter(Boolean).at(-1) ?? path;
  return {
    path,
    title: albumTitle(folder),
    catalog: catalogCode(folder),
    year: albumYear(folder),
    tracks,
    discs: [...new Set(tracks.map((t) => t.disc))],
    coverPath: pickCover(path, files, sortedDiscs.map((g) => ({ dir: g.dir, files: g.files }))),
    others,
    discDirs: sortedDiscs.map((g) => g.dir),
  };
}

export const formatDuration = (secs: number) => {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
};

/** Formato de áudio que o Rust está tocando (vem em `music://state`). */
export interface FormatInfo {
  codec: string;
  bits: number | null;
  sample_rate: number;
  lossless: boolean;
  /** Taxa do dispositivo quando foi preciso reamostrar. */
  resampled_to: number | null;
}

const khz = (hz: number) => `${(hz / 1000).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} kHz`;

/** `FLAC · 16 bit / 44,1 kHz · sem perdas` (`→ 48 kHz` se reamostrado). */
export function albumFormatLabel(f: FormatInfo): string {
  const rate = f.bits ? `${f.bits} bit / ${khz(f.sample_rate)}` : khz(f.sample_rate);
  const parts = [f.codec, rate];
  if (f.lossless) parts.push("sem perdas");
  return parts.join(" · ") + (f.resampled_to ? ` → ${khz(f.resampled_to)}` : "");
}
