export type MediaType = "ANIME" | "MANGA";

export interface Title {
  romaji?: string | null;
  english?: string | null;
  native?: string | null;
  userPreferred?: string | null;
}

export interface CoverImage {
  extraLarge?: string | null;
  large?: string | null;
  /** Cor dominante da capa (`#rrggbb`). */
  color?: string | null;
}

export interface FuzzyDate {
  year?: number | null;
  month?: number | null;
  day?: number | null;
}

/** Campos do fragment `Summary` (cards e casamento). */
export interface MediaSummary {
  id: number;
  type: MediaType;
  format: string | null;
  status: string | null;
  title: Title;
  synonyms: string[];
  seasonYear: number | null;
  startDate: FuzzyDate | null;
  coverImage: CoverImage | null;
  bannerImage: string | null;
  averageScore: number | null;
  popularity: number | null;
  genres: string[];
  episodes: number | null;
  chapters: number | null;
}

export interface Person {
  id: number;
  name: { full: string | null; native: string | null };
  image: { large: string | null } | null;
}

export interface MediaDetail extends MediaSummary {
  idMal: number | null;
  source: string | null;
  countryOfOrigin: string | null;
  siteUrl: string | null;
  description: string | null;
  season: "WINTER" | "SPRING" | "SUMMER" | "FALL" | null;
  endDate: FuzzyDate | null;
  duration: number | null;
  volumes: number | null;
  meanScore: number | null;
  favourites: number | null;
  tags: { name: string; rank: number; isMediaSpoiler: boolean }[];
  trailer: { id: string; site: string; thumbnail: string | null } | null;
  studios: { nodes: { id: number; name: string }[] };
  staff: { edges: { role: string; node: Person }[] };
  characters: { edges: { role: string; node: Person; voiceActors: Person[] }[] };
  relations: {
    edges: {
      relationType: string;
      node: {
        id: number;
        type: MediaType;
        format: string | null;
        title: { userPreferred: string | null };
        coverImage: CoverImage | null;
        startDate: FuzzyDate | null;
      };
    }[];
  };
  externalLinks: { site: string; url: string }[];
}
