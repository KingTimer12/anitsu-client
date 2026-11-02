import { describe, expect, test } from "vitest";
import { createMusicStore, DEFAULT_MUSIC_PREFS } from "./store";

function memory(initial: Record<string, string> = {}) {
  const m = new Map(Object.entries(initial));
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), dump: m };
}

describe("musicStore", () => {
  test("prefs default and merge", () => {
    const s = createMusicStore(memory(), 0);
    expect(s.prefs()).toEqual({ volume: 0.8, shuffle: false, repeat: "all", normalize: true });
    expect(DEFAULT_MUSIC_PREFS.volume).toBe(0.8);
    s.setPrefs({ volume: 0.5 });
    expect(s.prefs()).toEqual({ volume: 0.5, shuffle: false, repeat: "all", normalize: true });
  });

  test("progress roundtrip survives reload", async () => {
    const mem = memory();
    const s = createMusicStore(mem, 0);
    expect(s.progress("OSTs/A")).toBeUndefined();
    s.setProgress("OSTs/A", 2, 61.5);
    await new Promise((r) => setTimeout(r, 5));
    const again = createMusicStore(mem, 0);
    expect(again.progress("OSTs/A")).toMatchObject({ index: 2, position: 61.5 });
    expect(again.progress("OSTs/A")!.updatedAt).toBeGreaterThan(0);
  });

  test("corrupt storage falls back to defaults", () => {
    const s = createMusicStore(memory({ "higanbana:music:v1": "{não é json" }), 0);
    expect(s.prefs()).toEqual(DEFAULT_MUSIC_PREFS);
    expect(s.progress("x")).toBeUndefined();
  });

  test("storage that throws does not break", () => {
    const bad = {
      getItem: () => {
        throw new Error("bloqueado");
      },
      setItem: () => {
        throw new Error("cheio");
      },
    };
    const s = createMusicStore(bad, 0);
    s.setPrefs({ shuffle: true });
    expect(s.prefs().shuffle).toBe(true);
  });
});
