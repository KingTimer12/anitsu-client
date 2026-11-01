import { BookOpen, Clapperboard, Download, FolderOpen, Inbox, Music, Palette, Tv } from "lucide-solid";
import type { Component } from "solid-js";

export interface SectionMeta {
  label: string;
  blurb: string;
  icon: Component<{ class?: string }>;
  /** Classes de cor do bloco (ícone e brilho). */
  tint: string;
}

const META: Record<string, SectionMeta> = {
  Animes: { label: "Animes", blurb: "Séries, filmes e OVAs, por letra.", icon: Clapperboard, tint: "text-rose-300 bg-rose-500/12" },
  Mangás: { label: "Mangás", blurb: "Séries completas e volumes avulsos.", icon: BookOpen, tint: "text-sky-300 bg-sky-500/12" },
  Doramas: { label: "Doramas", blurb: "Dramas asiáticos com legenda.", icon: Tv, tint: "text-violet-300 bg-violet-500/12" },
  Desenhos: { label: "Desenhos", blurb: "Animações ocidentais.", icon: Palette, tint: "text-amber-300 bg-amber-500/12" },
  OSTs: { label: "OSTs", blurb: "Trilhas de animes, jogos e filmes.", icon: Music, tint: "text-emerald-300 bg-emerald-500/12" },
  "[#Temp]": { label: "Recentes", blurb: "Envios novos, antes de organizar.", icon: Inbox, tint: "text-slate-300 bg-slate-500/12" },
  "AniGUI (Downloader Oficial)": {
    label: "AniGUI",
    blurb: "O programa oficial de download.",
    icon: Download,
    tint: "text-orange-300 bg-orange-500/12",
  },
};

export const sectionMeta = (name: string): SectionMeta =>
  META[name] ?? { label: name, blurb: "", icon: FolderOpen, tint: "text-muted bg-white/6" };

/** Ordem no menu: conteúdo principal primeiro, utilitários no fim. */
const ORDER = ["Animes", "Mangás", "Doramas", "Desenhos", "OSTs", "[#Temp]", "AniGUI (Downloader Oficial)"];
export const sectionOrder = (a: string, b: string) => {
  const ia = ORDER.indexOf(a);
  const ib = ORDER.indexOf(b);
  return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib) || a.localeCompare(b, "pt-BR");
};
