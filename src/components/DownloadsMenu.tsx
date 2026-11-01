import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Check, Download, FolderOpen, RefreshCw, TriangleAlert } from "lucide-solid";
import { createSignal, For, Match, onCleanup, Show, Switch } from "solid-js";
import { downloads, progressOf, type Download as DownloadItem } from "../lib/downloads";
import { formatBytes } from "../lib/format";
import { Button, cx, ProgressRing } from "./ui";

function Row(props: { d: DownloadItem }) {
  const d = () => props.d;
  return (
    <li class="flex items-center gap-3 rounded-xl px-3 py-2.5 hover:bg-white/4">
      <span class="grid size-8 shrink-0 place-items-center rounded-lg bg-white/6">
        <Switch>
          <Match when={d().status === "done"}>
            <Check class="size-4 text-ok" aria-label="Concluído" />
          </Match>
          <Match when={d().status === "error"}>
            <TriangleAlert class="size-4 text-danger" aria-label="Falhou" />
          </Match>
          <Match when={true}>
            <Download class="size-4 text-muted" aria-hidden="true" />
          </Match>
        </Switch>
      </span>
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium" title={d().path}>
          {d().name}
        </p>
        <Show when={d().status === "running" || d().status === "queued"}>
          <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-white/10">
            <div
              class="h-full origin-left rounded-full bg-brand transition-transform duration-300"
              style={{ transform: `scaleX(${progressOf(d())})` }}
            />
          </div>
        </Show>
        <p class="mt-1 text-xs text-faint tabular-nums">
          <Switch>
            <Match when={d().status === "queued"}>Na fila</Match>
            <Match when={d().status === "running"}>
              {formatBytes(d().downloaded)}
              <Show when={d().total}> de {formatBytes(d().total!)}</Show>
            </Match>
            <Match when={d().status === "done"}>Concluído</Match>
            <Match when={d().status === "error"}>
              <span class="text-danger">{d().error}</span>
            </Match>
          </Switch>
        </p>
      </div>
      <Switch>
        <Match when={d().status === "done" && d().dest}>
          <button
            type="button"
            class="shrink-0 rounded-lg p-2 text-muted hover:bg-white/8 hover:text-fg"
            aria-label="Mostrar na pasta"
            title="Mostrar na pasta"
            onClick={() => revealItemInDir(d().dest!)}
          >
            <FolderOpen class="size-4" />
          </button>
        </Match>
        <Match when={d().status === "error"}>
          <button
            type="button"
            class="shrink-0 rounded-lg p-2 text-muted hover:bg-white/8 hover:text-fg"
            aria-label="Tentar de novo"
            title="Tentar de novo"
            onClick={() => downloads.start(d().path)}
          >
            <RefreshCw class="size-4" />
          </button>
        </Match>
      </Switch>
    </li>
  );
}

/** Botão de downloads na barra de título + painel suspenso. */
export function DownloadsMenu() {
  let root!: HTMLDivElement;
  const [open, setOpen] = createSignal(false);

  const onDocClick = (e: MouseEvent) => open() && !root.contains(e.target as Node) && setOpen(false);
  const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
  document.addEventListener("mousedown", onDocClick);
  document.addEventListener("keydown", onKey);
  onCleanup(() => {
    document.removeEventListener("mousedown", onDocClick);
    document.removeEventListener("keydown", onKey);
  });

  return (
    <div ref={root} class="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open()}
        aria-haspopup="dialog"
        aria-label={downloads.pending() ? `Downloads, ${downloads.pending()} em andamento` : "Downloads"}
        title="Downloads"
        class={cx(
          "relative inline-flex size-9 items-center justify-center rounded-lg text-muted transition hover:bg-white/8 hover:text-fg",
          open() && "bg-white/8 text-fg",
        )}
      >
        <Show when={downloads.pending()} fallback={<Download class="size-4.5" />}>
          <ProgressRing value={downloads.overall()} size={26} stroke={2.5}>
            <Download class="size-3.5 text-fg" />
          </ProgressRing>
          <span class="absolute -top-0.5 -right-0.5 grid h-4 min-w-4 place-items-center rounded-full bg-brand px-1 text-[10px] font-bold text-white">
            {downloads.pending()}
          </span>
        </Show>
      </button>

      <Show when={open()}>
        <section
          role="dialog"
          aria-label="Downloads"
          class="absolute top-11 right-0 w-96 origin-top-right animate-pop rounded-2xl border border-line-strong bg-elevated/95 shadow-2xl shadow-black/60 backdrop-blur-xl"
        >
          <header class="flex items-center justify-between border-b border-line px-4 py-3">
            <h2 class="font-display text-base font-semibold">Downloads</h2>
            <Show when={downloads.items.some((d) => d.status === "done" || d.status === "error")}>
              <Button variant="ghost" class="h-8 px-2 text-xs" onClick={downloads.clearFinished}>
                Limpar concluídos
              </Button>
            </Show>
          </header>
          <Show
            when={downloads.items.length}
            fallback={
              <div class="flex flex-col items-center gap-2 px-6 py-10 text-center">
                <Download class="size-8 text-faint" aria-hidden="true" />
                <p class="text-sm text-muted">Nenhum download ainda.</p>
                <p class="text-xs text-faint">Abra um anime e escolha os episódios para baixar.</p>
              </div>
            }
          >
            <ul class="scrollbar-thin max-h-[420px] overflow-y-auto p-1.5">
              <For each={downloads.items}>{(d) => <Row d={d} />}</For>
            </ul>
          </Show>
        </section>
      </Show>
    </div>
  );
}
