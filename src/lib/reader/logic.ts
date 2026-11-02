import type { FileEntry } from "../anitsu";

/** Lógica pura do leitor (sem Tauri): testada em `logic.test.ts`. */

export type Direction = "rtl" | "ltr";
export type ReadMode = "single" | "double" | "long" | "wide";
export type ProgressStyle = "thread" | "number" | "hidden";

export interface PageInfo {
  /** Posição no índice do zip (vai na URL da página). */
  index: number;
  name: string;
}

export const COMIC_EXT = new Set([".cbz", ".zip", ".cbr", ".rar"]);

export const isComic = (f: FileEntry) =>
  !f.is_directory && !!f.extension && COMIC_EXT.has(f.extension.toLowerCase());

const natural = (a: string, b: string) => a.localeCompare(b, "pt-BR", { numeric: true, sensitivity: "base" });

/** Só `.cbz`/`.zip`/`.cbr`/`.rar`, em ordem natural (igual à lista de arquivos). */
export function comicFiles(files: FileEntry[]): FileEntry[] {
  return files.filter(isComic).sort((a, b) => natural(a.name, b.name));
}

export function sortPages(pages: PageInfo[]): PageInfo[] {
  return [...pages].sort((a, b) => natural(a.name, b.name));
}

export interface ChapterLabel {
  kind: "Cap." | "Vol." | "";
  number: string | null;
  name?: string;
}

const CAP_RE = /\b(?:cap[íi]tulo|cap|chapter|ch|c)\.?\s*(\d+(?:\.\d+)?)/i;
const VOL_RE = /\b(?:volume|vol|v)\.?\s*(\d+(?:\.\d+)?)/i;
const tidy = (n: string) => String(Number(n));

/** "Cap. 12", "Vol. 3", só o número, ou o nome do arquivo. */
export function chapterLabel(fileName: string): ChapterLabel {
  // "_" separa palavras em nomes como "[Redisu]_AKTMP_V01_C_01".
  const base = fileName.replace(/\.[a-z0-9]{2,4}$/i, "").replace(/_/g, " ");
  // "Caps. 1-100": vários capítulos num arquivo só; o nome diz mais que um número.
  if (/\d+\s*[-–]\s*\d+/.test(base.replace(/\[[^\]]*\]|\([^)]*\)/g, ""))) return { kind: "", number: null, name: base.trim() };
  const cap = base.match(CAP_RE);
  if (cap) return { kind: "Cap.", number: tidy(cap[1]) };
  const vol = base.match(VOL_RE);
  if (vol) return { kind: "Vol.", number: tidy(vol[1]) };
  const bare = base.replace(/\[[^\]]*\]|\([^)]*\)/g, "").match(/(\d+(?:\.\d+)?)\s*$/);
  if (bare) return { kind: "", number: tidy(bare[1]) };
  return { kind: "", number: null, name: base.trim() };
}

export const labelText = (l: ChapterLabel) => (l.number === null ? (l.name ?? "") : `${l.kind} ${l.number}`.trim());

/**
 * Pares da página dupla, em ordem de leitura: capa sozinha, depois pares.
 * Página larga (imagem dupla já unida) fica sozinha e os pares recomeçam depois dela.
 */
export function buildSpreads(count: number, isWide: (i: number) => boolean): number[][] {
  const out: number[][] = [];
  let i = 0;
  while (i < count) {
    if (i === 0 || isWide(i) || i + 1 >= count || isWide(i + 1)) {
      out.push([i]);
      i += 1;
    } else {
      out.push([i, i + 1]);
      i += 2;
    }
  }
  return out;
}

export function spreadOf(spreads: number[][], page: number): number {
  const i = spreads.findIndex((s) => s.includes(page));
  return i >= 0 ? i : Math.max(0, spreads.length - 1);
}

/** Tecla → virar página, respeitando a direção de leitura. */
export function keyAction(key: string, dir: Direction): "next" | "prev" | null {
  if (key === " " || key === "PageDown") return "next";
  if (key === "PageUp") return "prev";
  if (key === "ArrowLeft") return dir === "rtl" ? "next" : "prev";
  if (key === "ArrowRight") return dir === "rtl" ? "prev" : "next";
  return null;
}

/** Quanto da pasta já foi lido (0–1): capítulos anteriores + parte do atual. */
export function stripProgress(chapter: number, chapters: number, page: number, pages: number): number {
  if (!chapters || !pages) return 0;
  return (chapter + Math.min(page + 1, pages) / pages) / chapters;
}

export interface OpenResult {
  pages: PageInfo[];
  /** `.cbz`/`.zip` guardados dentro do arquivo (coletâneas como "Caps. 1-100.zip"). */
  archives: string[];
}

/**
 * Unidade de leitura (um "capítulo" no índice): o arquivo inteiro, um `.cbz`
 * guardado dentro dele (`inner`) ou uma pasta dentro dele (`group`).
 */
export interface Unit {
  file: FileEntry;
  path: string;
  inner: string | null;
  group: string | null;
  /** Chave do progresso. Arquivo inteiro = o próprio caminho. */
  key: string;
  /** Nome mostrado (e de onde sai o "Cap. N"). */
  name: string;
}

export const unitKey = (path: string, inner: string | null, group: string | null) =>
  path + (inner ? `::${inner}` : "") + (group !== null ? `#${group}` : "");

const dirOf = (name: string) => (name.includes("/") ? name.slice(0, name.lastIndexOf("/")) : "");
const lastSeg = (p: string) => p.split("/").filter(Boolean).at(-1) ?? p;

export function baseUnit(path: string, file: FileEntry): Unit {
  return { file, path, inner: null, group: null, key: path, name: file.name };
}

/** Divide um arquivo em vários capítulos (zips internos ou pastas); `null` = é um capítulo só. */
export function expandUnits(path: string, file: FileEntry, r: OpenResult): Unit[] | null {
  if (!r.pages.length && r.archives.length) {
    return [...r.archives]
      .sort(natural)
      .map((inner) => ({ file, path, inner, group: null, key: unitKey(path, inner, null), name: lastSeg(inner) }));
  }
  const groups = [...new Set(r.pages.map((p) => dirOf(p.name)))].sort(natural);
  if (groups.length < 2) return null;
  return groups.map((group) => ({
    file,
    path,
    inner: null,
    group,
    key: unitKey(path, null, group),
    name: group ? lastSeg(group) : file.name,
  }));
}

export function pagesOfGroup(pages: PageInfo[], group: string | null): PageInfo[] {
  return group === null ? pages : pages.filter((p) => dirOf(p.name) === group);
}
