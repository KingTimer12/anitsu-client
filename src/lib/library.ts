import type { MediaType } from "./anilist/types";

/**
 * Seções da nuvem que têm correspondência no AniList e a profundidade
 * da pasta de título dentro delas.
 *
 *   Animes/Letra B/B - The Beginning               → título (profundidade 3)
 *   Mangás/Séries Mangás/Letra G/Gantz G (2015)    → título (profundidade 4)
 */
interface Section {
  prefix: string[];
  type: MediaType;
  titleDepth: number;
}

const SECTIONS: Section[] = [
  { prefix: ["Animes"], type: "ANIME", titleDepth: 3 },
  { prefix: ["Mangás", "Séries Mangás"], type: "MANGA", titleDepth: 4 },
];

export type Place =
  | { kind: "home" }
  /** Pasta comum (sem AniList). */
  | { kind: "folder"; path: string }
  /** Pasta cujos filhos são títulos → grade de pôsteres. */
  | { kind: "shelf"; path: string; type: MediaType }
  /** Dentro de um título. `rest` = subpastas abaixo do título. */
  | { kind: "title"; path: string; titlePath: string; rest: string[]; type: MediaType };

export const splitPath = (path: string) => path.split("/").filter(Boolean);
export const joinSegments = (segments: string[]) => segments.join("/");

export function classify(path: string): Place {
  const segs = splitPath(path);
  if (segs.length === 0) return { kind: "home" };

  for (const s of SECTIONS) {
    if (!s.prefix.every((p, i) => segs[i] === p)) continue;
    if (segs.length === s.titleDepth - 1) return { kind: "shelf", path, type: s.type };
    if (segs.length >= s.titleDepth) {
      return {
        kind: "title",
        path,
        titlePath: joinSegments(segs.slice(0, s.titleDepth)),
        rest: segs.slice(s.titleDepth),
        type: s.type,
      };
    }
  }
  return { kind: "folder", path };
}

/** Tipo de mídia AniList para resultados de busca da nuvem. */
export function mediaTypeForKind(kind: string): MediaType | null {
  if (kind === "anime") return "ANIME";
  if (kind === "manga") return "MANGA";
  return null;
}

/** `[2018] B - The Beginning` / `Gantz G (2015)` → 2018 / 2015. */
export function folderYear(name: string): number | null {
  const m = name.match(/[[(]((?:19|20)\d\d)[\])]/);
  return m ? Number(m[1]) : null;
}

/** Remove ano e tags de release: `[2018] B - The Beginning [DUAL]` → `B - The Beginning`. */
export function folderTitle(name: string): string {
  const t = name
    .replace(/\[[^\]]*\]|\([^)]*\)/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^[-_.\s]+|[-_.\s]+$/g, "");
  return t || name;
}

/** Tags de release entre colchetes: `[DUAL] [NF e Troidex] [BD 1080p]`. */
export function releaseTags(name: string): string[] {
  return [...name.matchAll(/\[([^\]]+)\]/g)].map((m) => m[1].trim()).filter((t) => !/^(19|20)\d\d$/.test(t));
}

export interface EpisodeName {
  number: string | null;
  label: string;
  /** Tags técnicas do arquivo: `BD-Rip 1080p H264 Dual`. */
  quality: string | null;
}

/** `[Troidex] B-The Beginning - Ep. 01 [BD-Rip_1080p_H264_Dual].mkv` → `01`. */
export function parseEpisode(fileName: string): EpisodeName {
  const base = fileName.replace(/\.[a-z0-9]{2,4}$/i, "");
  const m =
    base.match(/\b(?:Ep(?:is[oó]dio)?\.?|E)\s*(\d{1,4}(?:\.\d)?)(?:v\d)?\b/i) ??
    base.match(/\s-\s(\d{1,4}(?:\.\d)?)(?:v\d)?(?:\s|$|\[)/);
  const label = folderTitle(base);
  // Último grupo entre colchetes costuma ser a qualidade; o primeiro, o grupo de fansub.
  const groups = [...base.matchAll(/\[([^\]]+)\]/g)].map((g) => g[1]);
  const qualityRaw = groups.length > 1 ? groups.at(-1)! : (groups.find((g) => /\d{3,4}p|BD|WEB|x26[45]|H\.?26[45]/i.test(g)) ?? null);
  const quality = qualityRaw ? qualityRaw.replace(/[_.]+/g, " ").trim() : null;
  return { number: m ? m[1] : null, label, quality };
}
