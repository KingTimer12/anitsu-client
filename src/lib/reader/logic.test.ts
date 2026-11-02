import { describe, expect, it } from "vitest";
import type { FileEntry } from "../anitsu";
import {
  buildSpreads,
  chapterLabel,
  comicFiles,
  isComic,
  keyAction,
  sortPages,
  spreadOf,
  stripProgress,
  baseUnit,
  expandUnits,
  pagesOfGroup,
} from "./logic";

const f = (name: string, extra: Partial<FileEntry> = {}): FileEntry =>
  ({ name, is_directory: false, size: 1, extension: name.slice(name.lastIndexOf(".")), ...extra }) as FileEntry;

describe("isComic / comicFiles", () => {
  it("aceita cbz e zip, em qualquer caixa", () => {
    expect(isComic(f("a.CBZ"))).toBe(true);
    expect(isComic(f("a.zip"))).toBe(true);
    expect(isComic(f("a.cbr"))).toBe(true);
    expect(isComic(f("a.RAR"))).toBe(true);
    expect(isComic(f("a.pdf"))).toBe(false);
    expect(isComic(f("pasta.cbz", { is_directory: true }))).toBe(false);
  });
  it("ordena em ordem natural", () => {
    const names = comicFiles([f("Cap 10.cbz"), f("Cap 2.cbz"), f("capa.jpg"), f("Cap 1.cbz")]).map((x) => x.name);
    expect(names).toEqual(["Cap 1.cbz", "Cap 2.cbz", "Cap 10.cbz"]);
  });
});

describe("chapterLabel", () => {
  it("capítulo", () => {
    expect(chapterLabel("Chainsaw Man - Cap. 012.cbz")).toEqual({ kind: "Cap.", number: "12" });
    expect(chapterLabel("One Piece Capítulo 1050.5.cbz")).toEqual({ kind: "Cap.", number: "1050.5" });
    expect(chapterLabel("Berserk Ch.364.cbz")).toEqual({ kind: "Cap.", number: "364" });
  });
  it("volume", () => {
    expect(chapterLabel("Gantz G v03.cbz")).toEqual({ kind: "Vol.", number: "3" });
    expect(chapterLabel("Monster Volume 07 [Panini].cbz")).toEqual({ kind: "Vol.", number: "7" });
  });
  it("nomes com sublinhado: _C_01 e _V01_", () => {
    expect(chapterLabel("[Redisu]_AKTMP_V_01_C_01.rar")).toEqual({ kind: "Cap.", number: "1" });
    expect(chapterLabel("[Redisu]_AKTMP_V01_Completo.rar")).toEqual({ kind: "Vol.", number: "1" });
    expect(chapterLabel("Cardcaptor_v03.cbz")).toEqual({ kind: "Vol.", number: "3" });
  });
  it("só número no fim", () => {
    expect(chapterLabel("Dandadan 015.cbz")).toEqual({ kind: "", number: "15" });
  });
  it("intervalo (coletânea): nome sem extensão", () => {
    expect(chapterLabel("Caps. 1-100.zip")).toEqual({ kind: "", number: null, name: "Caps. 1-100" });
  });
  it("sem número: nome sem extensão", () => {
    expect(chapterLabel("Oneshot.cbz")).toEqual({ kind: "", number: null, name: "Oneshot" });
  });
});

describe("sortPages", () => {
  it("ordena por nome natural, mantendo o índice do zip", () => {
    const pages = [
      { index: 0, name: "p10.jpg" },
      { index: 1, name: "p2.jpg" },
      { index: 2, name: "p1.jpg" },
    ];
    expect(sortPages(pages).map((p) => p.index)).toEqual([2, 1, 0]);
  });
});

describe("buildSpreads", () => {
  it("capa sozinha, depois pares", () => {
    expect(buildSpreads(6, () => false)).toEqual([[0], [1, 2], [3, 4], [5]]);
  });
  it("página larga fica sozinha e realinha os pares", () => {
    expect(buildSpreads(7, (i) => i === 2)).toEqual([[0], [1], [2], [3, 4], [5, 6]]);
  });
  it("sem páginas", () => {
    expect(buildSpreads(0, () => false)).toEqual([]);
  });
  it("spreadOf acha o par de uma página", () => {
    const s = buildSpreads(6, () => false);
    expect(spreadOf(s, 4)).toBe(2);
    expect(spreadOf(s, 0)).toBe(0);
    expect(spreadOf(s, 99)).toBe(s.length - 1);
  });
});

describe("keyAction", () => {
  it("direita→esquerda: seta esquerda avança", () => {
    expect(keyAction("ArrowLeft", "rtl")).toBe("next");
    expect(keyAction("ArrowRight", "rtl")).toBe("prev");
  });
  it("esquerda→direita: seta direita avança", () => {
    expect(keyAction("ArrowRight", "ltr")).toBe("next");
    expect(keyAction("ArrowLeft", "ltr")).toBe("prev");
  });
  it("espaço e PageDown avançam; outras teclas, nada", () => {
    expect(keyAction(" ", "rtl")).toBe("next");
    expect(keyAction("PageUp", "ltr")).toBe("prev");
    expect(keyAction("x", "ltr")).toBeNull();
  });
});

describe("stripProgress", () => {
  it("fração da pasta: capítulos anteriores + parte do atual", () => {
    expect(stripProgress(0, 4, 0, 10)).toBeCloseTo(0.025);
    expect(stripProgress(2, 4, 9, 10)).toBeCloseTo(0.75);
    expect(stripProgress(0, 0, 0, 0)).toBe(0);
  });
});

describe("expandUnits", () => {
  const file = f("Caps. 1-100.zip");
  const path = "Beastars/Caps. 1-100.zip";

  it("coletânea: um capítulo por .cbz interno, em ordem natural", () => {
    const units = expandUnits(path, file, { pages: [], archives: ["Capitulo 10.cbz", "Capitulo 2.cbz", "Capitulo 1.cbz"] })!;
    expect(units.map((u) => u.inner)).toEqual(["Capitulo 1.cbz", "Capitulo 2.cbz", "Capitulo 10.cbz"]);
    expect(units[0].key).toBe(`${path}::Capitulo 1.cbz`);
    expect(chapterLabel(units[2].name)).toEqual({ kind: "Cap.", number: "10" });
  });

  it("pastas internas: um capítulo por pasta", () => {
    const pages = [
      { index: 0, name: "Vol 1/Cap 2/01.jpg" },
      { index: 1, name: "Vol 1/Cap 1/01.jpg" },
      { index: 2, name: "Vol 1/Cap 10/01.jpg" },
    ];
    const units = expandUnits(path, file, { pages, archives: [] })!;
    expect(units.map((u) => u.group)).toEqual(["Vol 1/Cap 1", "Vol 1/Cap 2", "Vol 1/Cap 10"]);
    expect(units.map((u) => u.name)).toEqual(["Cap 1", "Cap 2", "Cap 10"]);
    expect(pagesOfGroup(pages, "Vol 1/Cap 2").map((p) => p.index)).toEqual([0]);
  });

  it("uma pasta só (ou nenhuma): continua um capítulo", () => {
    expect(expandUnits(path, file, { pages: [{ index: 0, name: "Cap 1/01.jpg" }], archives: [] })).toBeNull();
    expect(expandUnits(path, file, { pages: [{ index: 0, name: "01.jpg" }], archives: [] })).toBeNull();
  });

  it("baseUnit usa o próprio caminho como chave", () => {
    expect(baseUnit(path, file)).toMatchObject({ key: path, inner: null, group: null, name: "Caps. 1-100.zip" });
  });
});
