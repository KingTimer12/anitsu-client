import { describe, expect, test } from "vitest";
import type { FileEntry } from "../anitsu";
import {
  albumFormatLabel,
  albumTitle,
  albumYear,
  buildAlbum,
  catalogCode,
  formatDuration,
  isAudio,
  isAlbumFolder,
  isDiscDir,
  parseTrack,
  pickCover,
} from "./logic";

const f = (name: string, size = 1): FileEntry => ({
  name,
  is_directory: false,
  size,
  modified: "2024-01-01T00:00:00",
  extension: name.includes(".") ? name.slice(name.lastIndexOf(".")).toLowerCase() : null,
});
const d = (name: string): FileEntry => ({ name, is_directory: true, size: 0, modified: "2024-01-01T00:00:00", extension: null });

describe("faixas", () => {
  test("isAudio", () => {
    expect(isAudio(f("01 - A.FLAC"))).toBe(true);
    expect(isAudio(f("a.opus"))).toBe(false);
    expect(isAudio(f("Disc 1.ape"))).toBe(false);
    expect(isAudio(d("x.flac"))).toBe(false);
  });

  test("parseTrack handles disc-track, dotted and bare names", () => {
    expect(parseTrack("1-03 - Action BGM (Thriller).flac")).toEqual({ disc: 1, number: 3, title: "Action BGM (Thriller)" });
    expect(parseTrack("01 - Morning Arch.flac")).toEqual({ disc: null, number: 1, title: "Morning Arch" });
    expect(parseTrack("01. Kimi wo Nosete.mp3")).toEqual({ disc: null, number: 1, title: "Kimi wo Nosete" });
    expect(parseTrack("Track 3.wav")).toEqual({ disc: null, number: 3, title: "Track 3" });
    expect(parseTrack("Candy Candy.MP3")).toEqual({ disc: null, number: null, title: "Candy Candy" });
    expect(parseTrack("1984-01-25 LOVEさりげなく.wav").number).toBeNull();
  });
});

describe("pasta do álbum", () => {
  test("catalogCode, year and title from folder names", () => {
    expect(catalogCode("2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)")).toBe("SVWC-7767");
    expect(catalogCode("1990.01.01 - Nostalgic TV Anime Theme Songs Vol 2 (FDLA-6235)")).toBe("FDLA-6235");
    expect(catalogCode("All That Jazz - Ghibli Jazz")).toBeNull();
    expect(albumYear("2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)")).toBe("2011");
    expect(albumYear("[Album] V.A. - Studio Ghibli no Uta [2008.11.26][APE]")).toBe("2008");
    expect(albumTitle("2011.05.25 - A-Channel TV1 OP1 - Morning Arch (SVWC-7767)")).toBe("A-Channel TV1 OP1 - Morning Arch");
    expect(albumTitle("-hack_Quantum_Main_Theme_Single_-_Shizuku_[Nanri_Yuuka]")).toBe("-hack Quantum Main Theme Single - Shizuku");
  });

  test("isDiscDir", () => {
    expect(isDiscDir("Disc 1")).toBe(1);
    expect(isDiscDir("CD2")).toBe(2);
    expect(isDiscDir("disco 3")).toBe(3);
    expect(isDiscDir("Scans")).toBeNull();
  });

  test("pickCover: named cover anywhere > first root image > first sub image > null", () => {
    expect(pickCover("A", [f("02.jpg"), f("01.jpg")], [{ dir: "A/Scans", files: [f("Front.JPG")] }])).toBe("A/Scans/Front.JPG");
    expect(pickCover("A", [f("folder.png"), f("01.jpg")])).toBe("A/folder.png");
    expect(pickCover("A", [f("10.jpg"), f("2.jpg")])).toBe("A/2.jpg");
    expect(pickCover("A", [], [{ dir: "A/Scans", files: [f("b.jpg"), f("a.jpg")] }])).toBe("A/Scans/a.jpg");
    expect(pickCover("A", [f("01.flac")])).toBeNull();
  });
});

describe("isAlbumFolder", () => {
  test("audio folders and disc-only folders are albums", () => {
    expect(isAlbumFolder([f("01 - A.flac"), f("cover.jpg")])).toBe(true);
    expect(isAlbumFolder([d("CD1"), d("CD2"), f("x.cue")])).toBe(true);
  });
  test("episode folders with a bonus track stay file lists", () => {
    expect(isAlbumFolder([f("Ep 01.mkv"), f("Ep 02.mkv"), f("OP.flac")])).toBe(false);
    expect(isAlbumFolder([f("Cap 01.cbz"), f("tema.mp3")])).toBe(false);
  });
  test("BD discs with videos are not albums", () => {
    expect(isAlbumFolder([d("Disc 1"), d("Disc 2")], [[f("01.mkv")], [f("02.mkv")]])).toBe(false);
    expect(isAlbumFolder([d("Disc 1"), d("Disc 2")])).toBe(true);
  });
  test("APE-only folders are not albums", () => {
    expect(isAlbumFolder([f("Disc 1.ape"), f("Disc 1.cue")])).toBe(false);
  });
});

describe("buildAlbum", () => {
  test("returns null without playable tracks", () => {
    expect(buildAlbum("x", [f("Disc 1.ape"), f("Disc 1.cue")])).toBeNull();
  });

  test("sorts naturally, groups discs from names and from disc dirs, keeps others", () => {
    const a = buildAlbum("OSTs/X/2009.11.18 - OST (AVCA-29415)", [
      f("1-10 - J.flac"),
      f("2-01 - K.flac"),
      f("1-02 - B.flac"),
      f("1-01 - A.flac"),
      f("x.cue"),
      d("Scans"),
    ])!;
    expect(a.tracks.map((t) => t.title)).toEqual(["A", "B", "J", "K"]);
    expect(a.tracks.map((t) => t.disc)).toEqual([1, 1, 1, 2]);
    expect(a.discs).toEqual([1, 2]);
    expect(a.catalog).toBe("AVCA-29415");
    expect(a.others.map((o) => o.name)).toEqual(["Scans", "x.cue"]);

    const b = buildAlbum("Y", [d("CD1"), d("CD2")], [
      { n: 2, dir: "Y/CD2", files: [f("01 - C.mp3")] },
      { n: 1, dir: "Y/CD1", files: [f("01 - A.mp3"), f("02 - B.mp3")] },
    ])!;
    expect(b.tracks.map((t) => [t.disc, t.title, t.path])).toEqual([
      [1, "A", "Y/CD1/01 - A.mp3"],
      [1, "B", "Y/CD1/02 - B.mp3"],
      [2, "C", "Y/CD2/01 - C.mp3"],
    ]);
    expect(b.discDirs).toEqual(["Y/CD1", "Y/CD2"]);

    const c = buildAlbum("Z", [f("01 - A.flac")])!;
    expect(c.discs).toEqual([1]);
  });

  test("track keeps size and extension", () => {
    const a = buildAlbum("Z", [f("01 - A.FLAC", 123)])!;
    expect(a.tracks[0]).toMatchObject({ path: "Z/01 - A.FLAC", size: 123, ext: ".flac", name: "01 - A.FLAC" });
  });
});

describe("formato", () => {
  test("formatDuration", () => {
    expect(formatDuration(271)).toBe("4:31");
    expect(formatDuration(3725)).toBe("1:02:05");
    expect(formatDuration(0)).toBe("0:00");
  });

  test("albumFormatLabel", () => {
    expect(albumFormatLabel({ codec: "FLAC", bits: 16, sample_rate: 44100, lossless: true, resampled_to: null })).toBe(
      "FLAC · 16 bit / 44,1 kHz · sem perdas",
    );
    expect(albumFormatLabel({ codec: "MP3", bits: null, sample_rate: 44100, lossless: false, resampled_to: null })).toBe("MP3 · 44,1 kHz");
    expect(albumFormatLabel({ codec: "FLAC", bits: 24, sample_rate: 96000, lossless: true, resampled_to: 48000 })).toBe(
      "FLAC · 24 bit / 96 kHz · sem perdas → 48 kHz",
    );
  });
});
