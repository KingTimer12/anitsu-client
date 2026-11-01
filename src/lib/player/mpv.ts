import {
  command as mpvCommand,
  destroy as mpvDestroy,
  getProperty,
  init,
  listenEvents,
  type MpvFormat,
  type MpvObservableProperty,
  observeProperties,
  setProperty,
} from "tauri-plugin-libmpv-api";

/** Propriedades do mpv que o player acompanha. */
export const OBSERVED = [
  ["time-pos", "double", "none"],
  ["duration", "double", "none"],
  ["pause", "flag"],
  ["volume", "double"],
  ["mute", "flag"],
  ["paused-for-cache", "flag"],
  ["demuxer-cache-time", "double", "none"],
  ["chapter-list", "node", "none"],
  ["track-list", "node", "none"],
  ["eof-reached", "flag", "none"],
] as const satisfies MpvObservableProperty[];

export interface MpvEvents {
  onProp(name: string, value: unknown): void;
  onEnd(reason: string): void;
  /** Arquivo aberto: duração, capítulos e faixas já disponíveis. */
  onLoaded(): void;
}

export interface Mpv {
  load(url: string, startSec?: number): Promise<void>;
  set(name: string, value: string | number | boolean): Promise<void>;
  get<T>(name: string, format?: MpvFormat): Promise<T>;
  command(...args: (string | number)[]): Promise<void>;
  destroy(): Promise<void>;
}

/**
 * Opções fixas: nada da interface do próprio mpv, cache só em RAM.
 * `extra` sobrescreve (ex.: `demuxer-max-bytes`).
 */
const BASE_OPTIONS: Record<string, string> = {
  vo: "gpu-next",
  hwdec: "auto-safe",
  "keep-open": "yes",
  "force-window": "yes",
  osc: "no",
  "osd-level": "0",
  "input-default-bindings": "no",
  "input-vo-keyboard": "no",
  cache: "yes",
  "cache-on-disk": "no",
  "demuxer-max-bytes": "150MiB",
  "demuxer-max-back-bytes": "50MiB",
  "sub-auto": "no",
};

export async function createMpv(extra: Record<string, string>, events: MpvEvents): Promise<Mpv> {
  await init({ initialOptions: { ...BASE_OPTIONS, ...extra }, observedProperties: OBSERVED });
  const unProps = await observeProperties(OBSERVED, ({ name, data }) => events.onProp(name, data));
  const unEvents = await listenEvents((e) => {
    if (e.event === "end-file") events.onEnd(e.reason);
    else if (e.event === "file-loaded") events.onLoaded();
  });

  return {
    async load(url, startSec) {
      const opts = startSec && startSec > 1 ? [`start=${startSec.toFixed(1)}`] : [];
      await mpvCommand("loadfile", [url, "replace", "-1", ...opts]);
    },
    set: (name, value) => setProperty(name, value),
    get: <T>(name: string, format: MpvFormat = "node") => getProperty(name, format) as Promise<T>,
    command: (name, ...args) => mpvCommand(String(name), args),
    async destroy() {
      unProps();
      unEvents();
      await mpvDestroy();
    },
  };
}
