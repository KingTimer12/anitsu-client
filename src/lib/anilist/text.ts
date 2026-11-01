import type { MediaSummary } from "./types";

/** Descrição do AniList (HTML simples) → parágrafos de texto + fonte, se houver. */
export function cleanDescription(raw: string | null | undefined): { paragraphs: string[]; source?: string } {
  if (!raw) return { paragraphs: [] };
  const withBreaks = raw.replace(/<br\s*\/?>/gi, "\n");
  const doc = new DOMParser().parseFromString(withBreaks, "text/html");
  const text = (doc.body.textContent ?? "").replace(/\r/g, "");

  let source: string | undefined;
  const body = text.replace(/\n?\s*\((?:Source|Fonte):\s*([^)]*)\)\s*$/i, (_, s: string) => {
    source = s.trim();
    return "";
  });

  const paragraphs = body
    .split(/\n\s*\n+/)
    .map((p) => p.replace(/\s*\n\s*/g, " ").trim())
    .filter(Boolean);
  return { paragraphs, source };
}

export const mediaTitle = (m: Pick<MediaSummary, "title">) =>
  m.title.userPreferred ?? m.title.romaji ?? m.title.english ?? m.title.native ?? "";

export const mediaYear = (m: Pick<MediaSummary, "seasonYear" | "startDate">) =>
  m.seasonYear ?? m.startDate?.year ?? null;

const FORMAT: Record<string, string> = {
  TV: "Série de TV",
  TV_SHORT: "TV curta",
  MOVIE: "Filme",
  SPECIAL: "Especial",
  OVA: "OVA",
  ONA: "ONA",
  MUSIC: "Clipe musical",
  MANGA: "Mangá",
  NOVEL: "Light novel",
  ONE_SHOT: "One-shot",
};

const STATUS: Record<string, string> = {
  FINISHED: "Finalizado",
  RELEASING: "Em lançamento",
  NOT_YET_RELEASED: "Ainda não lançado",
  CANCELLED: "Cancelado",
  HIATUS: "Em hiato",
};

const SEASON: Record<string, string> = {
  WINTER: "Inverno",
  SPRING: "Primavera",
  SUMMER: "Verão",
  FALL: "Outono",
};

const GENRE: Record<string, string> = {
  Action: "Ação",
  Adventure: "Aventura",
  Comedy: "Comédia",
  Drama: "Drama",
  Ecchi: "Ecchi",
  Fantasy: "Fantasia",
  Hentai: "Hentai",
  Horror: "Terror",
  "Mahou Shoujo": "Mahou Shoujo",
  Mecha: "Mecha",
  Music: "Música",
  Mystery: "Mistério",
  Psychological: "Psicológico",
  Romance: "Romance",
  "Sci-Fi": "Ficção científica",
  "Slice of Life": "Cotidiano",
  Sports: "Esportes",
  Supernatural: "Sobrenatural",
  Thriller: "Suspense",
};

const SOURCE: Record<string, string> = {
  ORIGINAL: "Original",
  MANGA: "Mangá",
  LIGHT_NOVEL: "Light novel",
  VISUAL_NOVEL: "Visual novel",
  VIDEO_GAME: "Jogo",
  NOVEL: "Romance",
  WEB_NOVEL: "Web novel",
  WEB_MANGA: "Web mangá",
  DOUJINSHI: "Doujinshi",
  ANIME: "Anime",
  GAME: "Jogo",
  COMIC: "Quadrinhos",
  LIVE_ACTION: "Live action",
  PICTURE_BOOK: "Livro ilustrado",
  MULTIMEDIA_PROJECT: "Projeto multimídia",
  OTHER: "Outro",
};

const RELATION: Record<string, string> = {
  SEQUEL: "Sequência",
  PREQUEL: "Prequela",
  SIDE_STORY: "História paralela",
  PARENT: "Obra principal",
  ALTERNATIVE: "Versão alternativa",
  ADAPTATION: "Adaptação",
  SOURCE: "Obra original",
  SUMMARY: "Resumo",
  SPIN_OFF: "Spin-off",
  CHARACTER: "Mesmos personagens",
  COMPILATION: "Compilação",
  CONTAINS: "Contém",
  OTHER: "Relacionado",
};

const ROLE: Record<string, string> = {
  Director: "Direção",
  "Original Creator": "Criação original",
  "Original Story": "História original",
  "Original Character Design": "Design original de personagens",
  "Character Design": "Design de personagens",
  "Series Composition": "Composição da série",
  Script: "Roteiro",
  Music: "Música",
  "Art Director": "Direção de arte",
  "Sound Director": "Direção de som",
  "Chief Animation Director": "Direção-chefe de animação",
  "Animation Director": "Direção de animação",
  "Assistant Director": "Assistente de direção",
  "Director of Photography": "Direção de fotografia",
  "Theme Song Performance": "Tema musical",
  Story: "História",
  Art: "Arte",
  "Story & Art": "História e arte",
};

const CHARACTER_ROLE: Record<string, string> = {
  MAIN: "Principal",
  SUPPORTING: "Coadjuvante",
  BACKGROUND: "Figurante",
};

const pick = (map: Record<string, string>) => (key: string | null | undefined) =>
  key ? (map[key] ?? key.charAt(0) + key.slice(1).toLowerCase().replace(/_/g, " ")) : "";

export const label = {
  format: pick(FORMAT),
  status: pick(STATUS),
  season: pick(SEASON),
  genre: (g: string) => GENRE[g] ?? g,
  source: pick(SOURCE),
  relation: pick(RELATION),
  characterRole: pick(CHARACTER_ROLE),
  /** "Key Animation (ep 3)" → mantém detalhes entre parênteses. */
  staffRole: (role: string) => {
    const m = role.match(/^([^(]+?)\s*(\(.*\))?$/);
    if (!m) return role;
    const base = ROLE[m[1]] ?? m[1];
    return m[2] ? `${base} ${m[2]}` : base;
  },
};
