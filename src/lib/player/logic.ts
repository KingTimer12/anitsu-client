import type { FileEntry } from "../anitsu";
import { parseEpisode } from "../library";

/** Lógica pura do player (sem mpv, sem Tauri): testada em `logic.test.ts`. */

export interface Interval {
  start: number;
  end: number;
}
export interface SkipTimes {
  op: Interval | null;
  ed: Interval | null;
}
export interface Chapter {
  title: string;
  time: number;
}
export interface Track {
  id: number;
  type: "sub" | "audio" | "video";
  lang?: string;
  title?: string;
  selected: boolean;
}
export interface TrackPref {
  subLang?: string;
  subTitle?: string;
  subOff: boolean;
  audioLang?: string;
  audioTitle?: string;
}

export const VIDEO_EXT = new Set([".mkv", ".mp4", ".avi", ".webm", ".m4v", ".mov", ".ts", ".wmv", ".flv"]);

// `(?![a-z])` em vez de `\b`: aceita "OP1"/"ED2"/"Opening1", recusa "Operation".
const OP_RE = /^(op|opening|intro|abertura)(?![a-z])/i;
const ED_RE = /^(ed|ending|outro|credits|encerramento)(?![a-z])/i;

/** Abertura/encerramento pelos títulos dos capítulos; cada um vai até o capítulo seguinte. */
export function classifyChapters(chapters: Chapter[], duration: number): SkipTimes {
  const sorted = [...chapters].sort((a, b) => a.time - b.time);
  const find = (re: RegExp): Interval | null => {
    const i = sorted.findIndex((c) => re.test(c.title.trim()));
    if (i < 0) return null;
    return { start: sorted[i].time, end: sorted[i + 1]?.time ?? duration };
  };
  return { op: find(OP_RE), ed: find(ED_RE) };
}

/** Número do episódio pelo nome do arquivo (`S01E03`, `Ep. 12`, ` - 06 [`, `_01_[`). */
export function episodeNumber(fileName: string): number | null {
  const base = fileName.replace(/\.[a-z0-9]{2,4}$/i, "");
  const se = base.match(/S\d+E(\d+)/i);
  if (se) return Number(se[1]);
  // ` - 06 [1080p]`: o número logo antes das tags vence títulos como "100 Years Quest".
  const dash = base.match(/\s-\s(\d{1,4})(?:v\d)?\s*(?=\[|$)/);
  if (dash) return Number(dash[1]);
  const parsed = parseEpisode(fileName).number;
  if (parsed) return Math.trunc(Number(parsed));
  const loose = base.match(/[_\s.]-?[_\s.]?(\d{1,4})(?!\d|p)[_\s.]*(?=\[|$)/i);
  return loose ? Number(loose[1]) : null;
}

const isVideo = (f: FileEntry) => !f.is_directory && !!f.extension && VIDEO_EXT.has(f.extension.toLowerCase());

/** Só vídeos, em ordem natural (igual à lista de arquivos). */
export function videoFiles(files: FileEntry[]): FileEntry[] {
  return files.filter(isVideo).sort((a, b) => a.name.localeCompare(b.name, "pt-BR", { numeric: true }));
}

export function nextEpisode(files: FileEntry[], currentName: string): FileEntry | null {
  const list = videoFiles(files);
  const i = list.findIndex((f) => f.name === currentName);
  return i >= 0 ? (list[i + 1] ?? null) : null;
}

/** Visto: 90% da duração ou começo do encerramento. */
export function isWatched(position: number, duration: number, ed: Interval | null): boolean {
  if (duration > 0 && position >= duration * 0.9) return true;
  return !!ed && position >= ed.start;
}

/** Quando mostrar o cartão de próximo episódio. */
export function nextCardStart(duration: number, ed: Interval | null, countdownSec: number): number {
  return ed?.start ?? duration - countdownSec;
}

const AUX_RE = /sign|song|forced/i;

/** ISO 639-1 → 639-2 dos idiomas comuns no acervo; arquivos usam os dois (e `pt-BR`). */
const ISO1_TO_2: Record<string, string> = {
  pt: "por", en: "eng", ja: "jpn", ko: "kor", es: "spa", zh: "chi", fr: "fre", de: "ger", it: "ita", ru: "rus", ar: "ara",
};
const ALIASES: Record<string, string> = { zho: "chi", fra: "fre", deu: "ger" };

export function normalizeLang(lang: string | undefined): string | undefined {
  if (!lang) return undefined;
  const base = lang.toLowerCase().split(/[-_]/)[0];
  return ISO1_TO_2[base] ?? ALIASES[base] ?? base;
}

function bestOf(tracks: Track[], lang: string | undefined, title?: string): Track | undefined {
  const want = normalizeLang(lang);
  if (!want) return undefined;
  const same = tracks.filter((t) => normalizeLang(t.lang) === want);
  return same.find((t) => title && t.title === title) ?? same.find((t) => !AUX_RE.test(t.title ?? "")) ?? same[0];
}

/**
 * Legenda/áudio a aplicar ao abrir um arquivo: escolha lembrada do release
 * (por idioma + título, nunca índice) e, sem ela, os idiomas preferidos.
 * `null` = deixar como o mpv escolheu.
 */
export function pickTracks(
  tracks: Track[],
  pref: TrackPref | undefined,
  defaults: { subLang: string; audioLang: string },
): { sid: number | "no" | null; aid: number | null } {
  const subs = tracks.filter((t) => t.type === "sub");
  const audios = tracks.filter((t) => t.type === "audio");

  let sid: number | "no" | null = null;
  if (pref?.subOff) sid = "no";
  else sid = (bestOf(subs, pref?.subLang, pref?.subTitle) ?? bestOf(subs, defaults.subLang))?.id ?? null;

  const aid = (bestOf(audios, pref?.audioLang, pref?.audioTitle) ?? bestOf(audios, defaults.audioLang))?.id ?? null;
  return { sid, aid };
}

export function formatTime(sec: number): string {
  const s = Math.max(0, Math.floor(sec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/**
 * Pode ir sozinho para o próximo episódio? Só com o arquivo atual carregado
 * (um `eof` atrasado do arquivo anterior não conta) e uma vez por arquivo.
 */
export function canAutoAdvance(s: {
  loaded: boolean;
  eof: boolean;
  autoNext: boolean;
  cancelled: boolean;
  hasNext: boolean;
  advancedFrom: string | null;
  current: string;
}): boolean {
  return s.loaded && s.eof && s.autoNext && !s.cancelled && s.hasNext && s.advancedFrom !== s.current;
}

const RETRY_DELAYS = [2000, 4000, 8000, 16000, 30000];

/** Espera antes da tentativa `attempt` (0, 1, ...); `null` = desistir. */
export function retryDelay(attempt: number): number | null {
  return RETRY_DELAYS[attempt] ?? null;
}
