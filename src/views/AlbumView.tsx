import "@fontsource/zen-old-mincho/700.css";
import "@fontsource/zen-old-mincho/900.css";
import { AudioLines, Pause, Play } from "lucide-solid";
import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import { FileList } from "../components/FileList";
import { cx } from "../components/ui";
import type { AlbumData } from "../lib/music/album";
import { musicApi } from "../lib/music/api";
import { albumFormatLabel, formatDuration, type Track } from "../lib/music/logic";
import { music } from "../lib/music/session";
import { musicStore } from "../lib/music/store";
import { router } from "../lib/router";
import { musicKeys } from "../components/music/MusicBar";

const screentone = "bg-[radial-gradient(rgb(232_40_60/0.18)_1px,transparent_1.4px)] bg-[length:5px_5px]";
const pad = (n: number | null, i: number) => String(n ?? i + 1).padStart(2, "0");

/** Encarte de CD japonês: capa com obi, lista de faixas em mincho. */
export function AlbumView(props: { data: AlbumData; work?: string }) {
  const a = () => props.data.album;
  const [tab, setTab] = createSignal<"tracks" | "files">("tracks");
  const [coverFailed, setCoverFailed] = createSignal(false);

  // Só quando o álbum muda (o estado do player muda 4×/s e não deve repetir isso).
  createEffect(on(a, (al) => music.show(al)));

  // Sem imagem na pasta: a capa embutida na primeira faixa (um pedido só).
  const [embedded, setEmbedded] = createSignal<string | null>(null);
  createEffect(
    on(
      () => a().path,
      () => {
        setEmbedded(null);
        if (props.data.cover || !a().tracks[0]) return;
        musicApi
          .meta(a().tracks[0], true)
          .then((m) => setEmbedded(m.cover))
          .catch(() => {});
      },
    ),
  );
  const coverSrc = () =>
    props.data.cover && !coverFailed() ? musicApi.imageUrl(props.data.cover.path, props.data.cover.size) : embedded();

  const current = () => music.isCurrent(a());
  const st = () => (current() ? music.state() : null);
  const progress = () => musicStore.progress(a().path);
  const obi = () => a().catalog ?? a().year;

  const format = createMemo(() => {
    const live = st()?.format;
    if (live) return albumFormatLabel(live);
    const m = a().tracks.map((t) => music.meta(t.path)).find(Boolean);
    return m ? albumFormatLabel({ codec: m.codec, bits: m.bits, sample_rate: m.sample_rate, lossless: m.lossless, resampled_to: null }) : null;
  });

  const totalSecs = createMemo(() => {
    const ds = a().tracks.map((t) => music.meta(t.path)?.duration);
    return ds.every((d) => d != null) ? ds.reduce((s, d) => s + d!, 0) : null;
  });

  const play = (i: number, at = 0) => music.open(a(), i, at);
  const resume = () => {
    const p = progress();
    return p && (p.index > 0 || p.position > 5) && p.index < a().tracks.length ? p : null;
  };

  const discOf = (d: number) => a().tracks.map((t, i) => ({ t, i })).filter((x) => x.t.disc === d);

  function Row(p: { t: Track; i: number }) {
    const meta = () => music.meta(p.t.path);
    const on = () => st()?.index === p.i;
    return (
      <li>
        <button
          type="button"
          onClick={() => (on() ? void musicApi.command("toggle") : play(p.i))}
          class={cx(
            "grid w-full grid-cols-[3rem_1fr_auto] items-center gap-3 rounded-md px-3 py-2 text-left transition-colors",
            on() ? "bg-brand/10" : "hover:bg-white/[0.04]",
          )}
          aria-current={on() ? "true" : undefined}
        >
          <span class={cx("font-mincho text-xl leading-none font-bold tabular-nums", on() ? "text-brand-ink" : "text-faint")}>
            <Show when={on() && st()?.playing} fallback={pad(p.t.number, p.i)}>
              <AudioLines class="size-5 animate-pulse" aria-label="Tocando" />
            </Show>
          </span>
          <span class="min-w-0">
            <span class={cx("block truncate", on() && "text-brand-ink")}>{meta()?.title || p.t.title}</span>
            <Show when={meta()?.artist}>
              <span class="block truncate text-xs text-muted">{meta()!.artist}</span>
            </Show>
          </span>
          <span class="text-sm text-muted tabular-nums">{meta()?.duration != null ? formatDuration(meta()!.duration!) : ""}</span>
        </button>
      </li>
    );
  }

  return (
    <div class="grid gap-10 md:grid-cols-[18rem_1fr]" onKeyDown={musicKeys}>
      <aside class={cx("self-start rounded-xl p-5 md:sticky md:top-6", screentone)}>
        <div class="relative mr-5">
          <div class="aspect-square overflow-hidden rounded-md bg-surface shadow-lg shadow-black/40">
            <Show
              when={coverSrc()}
              fallback={
                <div class="grid h-full place-items-center overflow-hidden p-4 text-center font-mincho text-xl font-bold text-muted [overflow-wrap:anywhere]">
                  {a().title}
                </div>
              }
            >
              {(src) => (
                <img
                  src={src()}
                  alt={`Capa de ${a().title}`}
                  class="size-full object-cover"
                  onError={() => (props.data.cover && !coverFailed() ? setCoverFailed(true) : setEmbedded(null))}
                />
              )}
            </Show>
          </div>
          <Show when={obi()}>
            <div
              class="absolute top-0 -right-5 flex h-full w-9 items-center justify-center rounded-r-md bg-brand font-mincho text-xs font-bold tracking-[0.2em] text-white shadow-md [writing-mode:vertical-rl]"
              aria-label={a().catalog ? `Código do disco ${a().catalog}` : `Ano ${a().year}`}
            >
              {obi()}
            </div>
          </Show>
        </div>

        <h1 class="mt-5 font-mincho text-2xl leading-snug font-bold [overflow-wrap:anywhere]">{a().title}</h1>
        <p class="mt-1 text-sm text-muted [overflow-wrap:anywhere]">{[props.work, a().year].filter(Boolean).join(" · ")}</p>

        <div class="mt-4 flex flex-wrap gap-2">
          <Show
            when={current() && st()?.playing}
            fallback={
              <button
                type="button"
                class="inline-flex h-10 items-center gap-2 rounded-lg bg-brand px-4 font-medium text-white transition hover:brightness-110 active:scale-95"
                onClick={() => (current() && !st()?.ended ? void musicApi.command("resume") : play(0))}
              >
                <Play class="size-4 fill-current" aria-hidden="true" />
                {current() && !st()?.ended ? "Continuar" : "Tocar"}
              </button>
            }
          >
            <button
              type="button"
              class="inline-flex h-10 items-center gap-2 rounded-lg bg-brand px-4 font-medium text-white transition hover:brightness-110 active:scale-95"
              onClick={() => void musicApi.command("pause")}
            >
              <Pause class="size-4 fill-current" aria-hidden="true" />
              Pausar
            </button>
          </Show>
          <Show when={!current() && resume()}>
            {(p) => (
              <button
                type="button"
                class="inline-flex h-10 items-center rounded-lg border border-line-strong px-3 text-sm font-medium transition hover:border-brand-ink"
                onClick={() => play(p().index, p().position)}
              >
                Continuar faixa {p().index + 1} ({formatDuration(p().position)})
              </button>
            )}
          </Show>
        </div>

        <Show when={format()}>
          <p class="mt-4 text-xs text-muted">{format()}</p>
        </Show>
        <p class="mt-1 text-xs text-faint">
          {a().tracks.length} faixas{totalSecs() ? ` · ${formatDuration(totalSecs()!)}` : ""}
        </p>
      </aside>

      <section class="min-w-0">
        <div class="mb-3 flex gap-5 border-b border-line" role="tablist">
          <For
            each={[
              ["tracks", "Faixas"],
              ["files", `Arquivos (${a().others.length})`],
            ] as const}
          >
            {([id, label]) => (
              <button
                type="button"
                role="tab"
                aria-selected={tab() === id}
                onClick={() => setTab(id)}
                class={cx("-mb-px border-b-2 pb-2 text-sm font-medium transition-colors", tab() === id ? "border-brand text-fg" : "border-transparent text-muted hover:text-fg")}
              >
                {label}
              </button>
            )}
          </For>
        </div>

        <Show
          when={tab() === "tracks"}
          fallback={
            <Show when={a().others.length} fallback={<p class="py-8 text-sm text-muted">Só as faixas nesta pasta.</p>}>
              <FileList dir={a().path} files={a().others} onOpenFolder={router.go} />
            </Show>
          }
        >
          <For each={a().discs}>
            {(d) => (
              <>
                <Show when={a().discs.length > 1}>
                  <h2 class="mt-4 mb-1 px-3 font-mincho text-sm font-bold tracking-[0.15em] text-muted">Disco {d}</h2>
                </Show>
                <ol>
                  <For each={discOf(d)}>{(x) => <Row t={x.t} i={x.i} />}</For>
                </ol>
              </>
            )}
          </For>
        </Show>
      </section>
    </div>
  );
}
