import { describe, expect, it } from "vitest";
import type { FileEntry } from "../anitsu";
import {
  canAutoAdvance,
  classifyChapters,
  retryDelay,
  episodeNumber,
  formatTime,
  isWatched,
  nextCardStart,
  nextEpisode,
  pickTracks,
  type Track,
} from "./logic";

const ch = (title: string, time: number) => ({ title, time });

describe("classifyChapters", () => {
  it("abertura e encerramento com Avant/Part/Preview", () => {
    const r = classifyChapters(
      [ch("Avant", 0), ch("Opening", 62), ch("Part A", 152), ch("Part B", 700), ch("Ending", 1290), ch("Preview", 1380)],
      1420,
    );
    expect(r).toEqual({ op: { start: 62, end: 152 }, ed: { start: 1290, end: 1380 } });
  });
  it("Intro/Credits, encerramento vai até o fim", () => {
    expect(classifyChapters([ch("Intro", 0), ch("Episode", 90), ch("Credits", 1300)], 1400)).toEqual({
      op: { start: 0, end: 90 },
      ed: { start: 1300, end: 1400 },
    });
  });
  it("op/ed em minúsculas", () => {
    expect(classifyChapters([ch("op", 0), ch("main", 90), ch("ed", 1300)], 1400)).toEqual({
      op: { start: 0, end: 90 },
      ed: { start: 1300, end: 1400 },
    });
  });
  it("OP1/ED2/Opening1 com número colado", () => {
    expect(classifyChapters([ch("Prologue", 0), ch("OP1", 30), ch("A", 120), ch("ED2", 1300)], 1400)).toEqual({
      op: { start: 30, end: 120 },
      ed: { start: 1300, end: 1400 },
    });
    expect(classifyChapters([ch("Opening1", 0), ch("Main", 90)], 1400).op).toEqual({ start: 0, end: 90 });
  });
  it("palavras que só começam igual não contam (Operation, Education)", () => {
    expect(classifyChapters([ch("Operation", 0), ch("Education", 600)], 1400)).toEqual({ op: null, ed: null });
  });
  it("sem capítulos", () => {
    expect(classifyChapters([], 1400)).toEqual({ op: null, ed: null });
  });
  it("capítulos genéricos não contam", () => {
    expect(classifyChapters([ch("Chapter 1", 0), ch("Chapter 2", 600)], 1400)).toEqual({ op: null, ed: null });
  });
});

describe("episodeNumber", () => {
  it.each([
    ["[FênixFansub_&_IsekaiSubs]_Fairy_Gone_01_[HDTV][1080p][8bits][877F5B35].mp4", 1],
    ["[Erai-raws] Fairy Tail - 100 Years Quest - 06 [1080p][HEVC][Multiple Subtitle][F52256A9].mkv", 6],
    ["Zombie.Detective.S01E03.1080p.NF.WEB-DL.DDP2.0.x264-HBO.mkv", 3],
    ["Naruto Ep. 012.mkv", 12],
    ["[Grupo] Filme [1080p].mkv", null],
    ["[Grupo] Titulo - 1080p.mkv", null],
  ])("%s → %s", (name, n) => {
    expect(episodeNumber(name)).toBe(n);
  });
});

const file = (name: string, is_directory = false): FileEntry => ({
  name,
  is_directory,
  size: 1,
  modified: "",
  extension: is_directory ? null : name.slice(name.lastIndexOf(".")),
});

describe("nextEpisode", () => {
  const files = [file("Ep 10.mkv"), file("Ep 2.mkv"), file("Ep 1.mkv"), file("Ep 1.ass"), file("notas.txt"), file("Extras", true)];
  it("ordem natural, ignora o que não é vídeo", () => {
    expect(nextEpisode(files, "Ep 1.mkv")?.name).toBe("Ep 2.mkv");
    expect(nextEpisode(files, "Ep 2.mkv")?.name).toBe("Ep 10.mkv");
  });
  it("último episódio → null", () => {
    expect(nextEpisode(files, "Ep 10.mkv")).toBeNull();
  });
});

describe("isWatched", () => {
  it("90% ou início do encerramento", () => {
    expect(isWatched(1200, 1400, null)).toBe(false);
    expect(isWatched(1260, 1400, null)).toBe(true);
    expect(isWatched(1291, 1400, { start: 1290, end: 1380 })).toBe(true);
  });
});

describe("nextCardStart", () => {
  it("no encerramento ou nos últimos segundos", () => {
    expect(nextCardStart(1400, { start: 1290, end: 1380 }, 5)).toBe(1290);
    expect(nextCardStart(1400, null, 10)).toBe(1390);
  });
});

describe("pickTracks", () => {
  const tracks: Track[] = [
    { id: 1, type: "sub", lang: "eng", title: "English", selected: false },
    { id: 2, type: "sub", lang: "por", title: "Português (Brasil)", selected: false },
    { id: 3, type: "sub", lang: "por", title: "Signs", selected: false },
    { id: 1, type: "audio", lang: "jpn", selected: true },
    { id: 2, type: "audio", lang: "por", selected: false },
  ];
  const defaults = { subLang: "por", audioLang: "jpn" };
  it("padrões, evitando faixa de placas", () => {
    expect(pickTracks(tracks, undefined, defaults)).toEqual({ sid: 2, aid: 1 });
  });
  it("legenda desligada lembrada", () => {
    expect(pickTracks(tracks, { subOff: true }, defaults).sid).toBe("no");
  });
  it("idioma e título lembrados", () => {
    expect(pickTracks(tracks, { subOff: false, subLang: "eng", subTitle: "English" }, defaults).sid).toBe(1);
  });
  it("códigos de 2 letras e com região (pt, pt-BR, en) batem com por/eng", () => {
    const short: Track[] = [
      { id: 1, type: "sub", lang: "en", title: "English", selected: true },
      { id: 2, type: "sub", lang: "pt-BR", title: "Portuguese(Brazil)", selected: false },
      { id: 3, type: "sub", lang: "es", title: "Spanish(Latin_America)", selected: false },
      { id: 1, type: "audio", lang: "ja", selected: true },
    ];
    expect(pickTracks(short, undefined, defaults)).toEqual({ sid: 2, aid: 1 });
  });
  it("idioma lembrado inexistente cai no padrão", () => {
    expect(pickTracks(tracks, { subOff: false, subLang: "spa" }, defaults).sid).toBe(2);
  });
});

describe("formatTime", () => {
  it("m:ss e h:mm:ss", () => {
    expect(formatTime(75)).toBe("1:15");
    expect(formatTime(3725)).toBe("1:02:05");
    expect(formatTime(0)).toBe("0:00");
  });
});

describe("canAutoAdvance", () => {
  const base = { loaded: true, eof: true, autoNext: true, cancelled: false, hasNext: true, advancedFrom: null, current: "ep1" };
  it("avança no fim do arquivo carregado", () => {
    expect(canAutoAdvance(base)).toBe(true);
  });
  it("não avança com arquivo novo ainda carregando (eof atrasado do anterior)", () => {
    expect(canAutoAdvance({ ...base, loaded: false })).toBe(false);
  });
  it("não avança duas vezes a partir do mesmo arquivo", () => {
    expect(canAutoAdvance({ ...base, advancedFrom: "ep1" })).toBe(false);
  });
  it("respeita cancelar e a configuração", () => {
    expect(canAutoAdvance({ ...base, cancelled: true })).toBe(false);
    expect(canAutoAdvance({ ...base, autoNext: false })).toBe(false);
    expect(canAutoAdvance({ ...base, hasNext: false })).toBe(false);
  });
});

describe("retryDelay", () => {
  it("cresce e para depois de 5 tentativas", () => {
    expect([0, 1, 2, 3, 4].map(retryDelay)).toEqual([2000, 4000, 8000, 16000, 30000]);
    expect(retryDelay(5)).toBeNull();
  });
});
