import { expect, test } from "vitest";
import type { Album, Track } from "./logic";
import { createMetaQueue } from "./metaQueue";

const track = (p: string): Track => ({ path: p, size: 1, name: p, disc: 1, number: null, title: p, ext: ".flac" });
const album = (path: string, n: number): Album => ({
  path,
  title: path,
  catalog: null,
  year: null,
  tracks: Array.from({ length: n }, (_, i) => track(`${path}/${i}.flac`)),
  discs: [1],
  coverPath: null,
  others: [],
  discDirs: [],
});

function rig() {
  const done = new Map<string, number>();
  let active = 0;
  let maxActive = 0;
  const calls: string[] = [];
  const q = createMetaQueue(
    async (t: Track) => {
      calls.push(t.path);
      active++;
      maxActive = Math.max(maxActive, active);
      await new Promise((r) => setTimeout(r, 5));
      active--;
      return 1;
    },
    { has: (p) => done.has(p), set: (p, v) => void done.set(p, v) },
  );
  return { q, calls, done, max: () => maxActive };
}

test("repeated loads of the same album fetch each track once, one at a time", async () => {
  const r = rig();
  const a = album("A", 4);
  const runs = [r.q.load(a), r.q.load(a), r.q.load(a), r.q.load(a)];
  await Promise.all(runs);
  expect(r.calls).toEqual(["A/0.flac", "A/1.flac", "A/2.flac", "A/3.flac"]);
  expect(r.max()).toBe(1);
  expect(r.done.size).toBe(4);
});

test("switching album stops the old one", async () => {
  const r = rig();
  const first = r.q.load(album("A", 5));
  await new Promise((res) => setTimeout(res, 7));
  await Promise.all([first, r.q.load(album("B", 2))]);
  expect(r.calls.filter((c) => c.startsWith("A/")).length).toBeLessThan(5);
  expect(r.calls.filter((c) => c.startsWith("B/"))).toEqual(["B/0.flac", "B/1.flac"]);
  expect(r.max()).toBe(1);
});
