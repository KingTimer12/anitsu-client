import { AudioLines, LoaderCircle, Pause, Play, Repeat, Repeat1, Shuffle, SkipBack, SkipForward, Volume1, Volume2, VolumeX } from "lucide-solid";
import { createEffect, createSignal, on, Show } from "solid-js";
import { musicApi } from "../../lib/music/api";
import { albumTitle, formatDuration } from "../../lib/music/logic";
import { music } from "../../lib/music/session";
import { router } from "../../lib/router";
import { toast } from "../../lib/toast";
import { cx } from "../ui";
import { Thread } from "./Thread";

const btn =
  "grid size-9 place-items-center rounded-md text-muted transition-colors hover:bg-white/5 hover:text-fg focus-visible:text-fg active:scale-95 disabled:opacity-40";

/** Atalhos do player: espaço pausa, ←/→ ±5 s. Ignora campos de texto. */
export function musicKeys(e: KeyboardEvent) {
  const s = music.state();
  const t = e.target as HTMLElement | null;
  if (!s?.album || t?.closest("input, textarea, select, [contenteditable]")) return;
  if (e.key === " ") {
    e.preventDefault();
    void musicApi.command("toggle");
  } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
    e.preventDefault();
    void musicApi.seek(Math.max(0, s.position + (e.key === "ArrowRight" ? 5 : -5)));
  }
}

/** Barra fixa embaixo do app: continua tocando enquanto se navega. */
export function MusicBar() {
  const s = () => music.state()!;
  const track = () => {
    const a = music.album();
    return a && s().index !== null ? a.tracks[s().index!] : undefined;
  };
  const title = () => {
    const t = track();
    return (t && music.meta(t.path)?.title) || s().title || t?.title || "";
  };
  /** Nome da obra (pasta acima do álbum); se for só um índice ("A", "0-9"), o do álbum. */
  const work = () => {
    const segs = (s().album ?? "").split("/").filter(Boolean);
    const parent = segs.at(-2) ?? "";
    return albumTitle(parent.length > 3 && !/^letra\s/i.test(parent) ? parent : (segs.at(-1) ?? ""));
  };
  const [muted, setMuted] = createSignal<number | null>(null);

  createEffect(
    on(
      () => s().error,
      (e) => e && toast.show(e, "error"),
    ),
  );

  const toggleMute = () => {
    const m = muted();
    if (m !== null) {
      setMuted(null);
      void musicApi.set({ volume: m });
    } else {
      setMuted(s().volume);
      void musicApi.set({ volume: 0 });
    }
  };
  const nextRepeat = () => (s().repeat === "off" ? "all" : s().repeat === "all" ? "one" : "off");

  return (
    <div
      class="flex h-16 shrink-0 items-center gap-4 border-t border-line bg-surface px-4"
      role="region"
      aria-label="Player de músicas"
      onKeyDown={(e) => {
        if (e.key.toLowerCase() === "m" && !(e.target as HTMLElement).closest("input")) toggleMute();
        else musicKeys(e);
      }}
    >
      <button
        type="button"
        class="grid size-10 shrink-0 place-items-center rounded-full bg-brand text-white transition hover:brightness-110 active:scale-95"
        aria-label={s().playing ? "Pausar" : "Tocar"}
        onClick={() => void musicApi.command("toggle")}
      >
        <Show when={!s().loading} fallback={<LoaderCircle class="size-4 animate-spin" aria-hidden="true" />}>
          <Show when={s().playing} fallback={<Play class="size-4 translate-x-px fill-current" aria-hidden="true" />}>
            <Pause class="size-4 fill-current" aria-hidden="true" />
          </Show>
        </Show>
      </button>

      <button
        type="button"
        class="w-52 min-w-0 shrink-0 text-left"
        onClick={() => s().album && router.go(s().album!)}
        title="Abrir o álbum"
      >
        <span class="block truncate text-sm font-medium">{title()}</span>
        <span class="block truncate text-xs text-muted">{work()}</span>
      </button>

      <div class="flex min-w-0 flex-1 items-center gap-3">
        <span class="w-11 text-right text-xs text-muted tabular-nums">{formatDuration(s().position)}</span>
        <Thread position={s().position} duration={s().duration} buffered={s().buffered} onSeek={(x) => void musicApi.seek(x)} />
        <span class="w-11 text-xs text-muted tabular-nums">{s().duration ? formatDuration(s().duration!) : "–:––"}</span>
      </div>

      <div class="flex shrink-0 items-center gap-0.5">
        <button type="button" class={btn} aria-label="Anterior" onClick={() => void musicApi.command("prev")}>
          <SkipBack class="size-4" aria-hidden="true" />
        </button>
        <button type="button" class={btn} aria-label="Próxima" onClick={() => void musicApi.command("next")}>
          <SkipForward class="size-4" aria-hidden="true" />
        </button>
        <button
          type="button"
          class={cx(btn, s().shuffle && "text-brand-ink")}
          aria-label="Aleatório"
          aria-pressed={s().shuffle}
          onClick={() => void musicApi.set({ shuffle: !s().shuffle })}
        >
          <Shuffle class="size-4" aria-hidden="true" />
        </button>
        <button
          type="button"
          class={cx(btn, s().repeat !== "off" && "text-brand-ink")}
          aria-label={s().repeat === "one" ? "Repetir faixa" : s().repeat === "all" ? "Repetir álbum" : "Não repetir"}
          title={s().repeat === "one" ? "Repetindo a faixa" : s().repeat === "all" ? "Repetindo o álbum" : "Sem repetir"}
          onClick={() => void musicApi.set({ repeat: nextRepeat() })}
        >
          <Show when={s().repeat === "one"} fallback={<Repeat class="size-4" aria-hidden="true" />}>
            <Repeat1 class="size-4" aria-hidden="true" />
          </Show>
        </button>
        <button
          type="button"
          class={cx(btn, s().normalize && "text-brand-ink")}
          aria-label="Volume igualado"
          aria-pressed={s().normalize}
          title={
            s().normalize
              ? "Volume igualado entre álbuns (vale a partir da próxima faixa)"
              : "Volume original dos arquivos (vale a partir da próxima faixa)"
          }
          onClick={() => void musicApi.set({ normalize: !s().normalize })}
        >
          <AudioLines class="size-4" aria-hidden="true" />
        </button>
        <button type="button" class={btn} aria-label={muted() !== null ? "Tirar do mudo" : "Mudo"} onClick={toggleMute}>
          <Show when={s().volume > 0} fallback={<VolumeX class="size-4" aria-hidden="true" />}>
            <Show when={s().volume > 0.5} fallback={<Volume1 class="size-4" aria-hidden="true" />}>
              <Volume2 class="size-4" aria-hidden="true" />
            </Show>
          </Show>
        </button>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={s().volume}
          aria-label="Volume"
          class="w-24 accent-[var(--color-brand)]"
          onInput={(e) => {
            setMuted(null);
            void musicApi.set({ volume: Number(e.currentTarget.value) });
          }}
        />
      </div>
    </div>
  );
}
