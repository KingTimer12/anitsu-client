import { createResource } from "solid-js";
import { type FileEntry, joinPath, nuvem } from "../anitsu";
import { type Album, buildAlbum, isAlbumFolder, isDiscDir, pickCover } from "./logic";

/** Subpastas que costumam ter a capa (só estas são listadas, uma por vez). */
const ART_DIR = /scan|cover|capa|art|book|jacket|image|img/i;
const MAX_ART_DIRS = 2;

export interface AlbumData {
  album: Album;
  cover: { path: string; size: number } | null;
}

const discDirs = (files: FileEntry[]) => files.filter((f) => f.is_directory && isDiscDir(f.name) !== null);

/** A pasta parece um álbum (confirmado em `loadAlbum` depois de listar os discos). */
export const looksLikeAlbum = (files: FileEntry[]) => isAlbumFolder(files);

/**
 * Monta o álbum da pasta: lista os discos (`CD1`, `Disc 2`) e até duas pastas de arte,
 * **uma por vez** (a nuvem limita pedidos seguidos). `null` se não houver nada tocável.
 */
export async function loadAlbum(path: string, files: FileEntry[]): Promise<AlbumData | null> {
  if (!looksLikeAlbum(files)) return null;
  const discs: { n: number; dir: string; files: FileEntry[] }[] = [];
  for (const d of discDirs(files)) {
    const dir = joinPath(path, d.name);
    const l = await nuvem.list(dir).catch(() => null);
    if (l) discs.push({ n: isDiscDir(d.name)!, dir, files: l.files });
  }
  if (!isAlbumFolder(files, discs.map((d) => d.files))) return null;
  const album = buildAlbum(path, files, discs);
  if (!album) return null;

  const groups = [{ dir: path, files }, ...discs.map((d) => ({ dir: d.dir, files: d.files }))];
  const hasNamed = groups.some((g) => g.files.some((f) => !f.is_directory && /^(cover|front|folder)\b/i.test(f.name)));
  if (!hasNamed) {
    for (const d of files.filter((f) => f.is_directory && ART_DIR.test(f.name)).slice(0, MAX_ART_DIRS)) {
      const dir = joinPath(path, d.name);
      const l = await nuvem.list(dir).catch(() => null);
      if (l) groups.push({ dir, files: l.files });
    }
  }
  const coverPath = pickCover(path, files, groups.slice(1));
  album.coverPath = coverPath;
  const entry = coverPath ? groups.flatMap((g) => g.files.map((f) => ({ path: joinPath(g.dir, f.name), size: f.size }))).find((e) => e.path === coverPath) : null;
  return { album, cover: entry ?? null };
}

/** Recurso Solid para a pasta atual. */
export function createAlbum(source: () => { path: string; files: FileEntry[] } | undefined) {
  return createResource(source, (s) => loadAlbum(s.path, s.files));
}
