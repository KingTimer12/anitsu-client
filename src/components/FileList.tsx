import { Check, Download, FileVideo, Folder, RefreshCw } from "lucide-solid";
import { createMemo, For, Match, Show, Switch } from "solid-js";
import type { FileEntry } from "../lib/anitsu";
import { joinPath } from "../lib/anitsu";
import { downloads, progressOf } from "../lib/downloads";
import { formatBytes } from "../lib/format";
import { parseEpisode } from "../lib/library";
import { cx, ProgressRing } from "./ui";

export function DownloadButton(props: { path: string; name: string }) {
  const d = () => downloads.byPath(props.path);
  return (
    <Switch
      fallback={
        <button
          type="button"
          onClick={() => downloads.start(props.path)}
          aria-label={`Baixar ${props.name}`}
          class="inline-flex h-9 items-center gap-2 rounded-lg border border-line-strong px-3 text-sm font-medium text-fg transition duration-150 hover:border-accent hover:bg-accent-soft active:scale-95"
        >
          <Download class="size-4" aria-hidden="true" />
          Baixar
        </button>
      }
    >
      <Match when={d()?.status === "queued"}>
        <span class="inline-flex h-9 items-center px-3 text-sm text-faint">Na fila</span>
      </Match>
      <Match when={d()?.status === "running"}>
        <ProgressRing value={progressOf(d()!)} size={36} label={`Baixando ${props.name}`}>
          <span class="text-[10px] font-bold tabular-nums">{Math.round(progressOf(d()!) * 100)}</span>
        </ProgressRing>
      </Match>
      <Match when={d()?.status === "done"}>
        <span class="inline-flex h-9 items-center gap-1.5 px-2 text-sm font-medium text-ok">
          <Check class="size-4" aria-hidden="true" /> Baixado
        </span>
      </Match>
      <Match when={d()?.status === "error"}>
        <button
          type="button"
          onClick={() => downloads.start(props.path)}
          title={d()!.error}
          class="inline-flex h-9 items-center gap-2 rounded-lg px-3 text-sm font-medium text-danger hover:bg-danger/10"
        >
          <RefreshCw class="size-4" aria-hidden="true" /> Tentar de novo
        </button>
      </Match>
    </Switch>
  );
}

/** Caminhos dos arquivos (não pastas) de uma listagem. */
export const filePaths = (dir: string, files: FileEntry[]) =>
  files.filter((f) => !f.is_directory).map((f) => joinPath(dir, f.name));

/**
 * Lista de uma pasta: subpastas primeiro, depois arquivos como episódios
 * (número grande quando o nome segue o padrão `Ep. 01`).
 */
export function FileList(props: { dir: string; files: FileEntry[]; onOpenFolder: (path: string) => void }) {
  const sorted = createMemo(() =>
    [...props.files].sort(
      (a, b) =>
        Number(b.is_directory) - Number(a.is_directory) || a.name.localeCompare(b.name, "pt-BR", { numeric: true }),
    ),
  );

  // Quando todos os episódios têm a mesma qualidade, mostra uma vez só no topo.
  const sharedQuality = createMemo(() => {
    const eps = sorted()
      .filter((f) => !f.is_directory)
      .map((f) => parseEpisode(f.name))
      .filter((e) => e.number);
    if (eps.length < 2) return null;
    const q = eps[0].quality;
    return q && eps.every((e) => e.quality === q) ? q : null;
  });

  return (
    <Show
      when={sorted().length}
      fallback={<p class="rounded-2xl border border-dashed border-line px-6 py-10 text-center text-sm text-muted">Esta pasta está vazia.</p>}
    >
      <Show when={sharedQuality()}>
        <p class="mb-2 text-sm text-muted">
          Todos os episódios em <span class="text-fg">{sharedQuality()}</span>
        </p>
      </Show>
      <ul class="divide-y divide-line overflow-hidden rounded-2xl border border-line bg-surface/60">
        <For each={sorted()}>
          {(f) => {
            const path = joinPath(props.dir, f.name);
            if (f.is_directory) {
              return (
                <li>
                  <button
                    type="button"
                    onClick={() => props.onOpenFolder(path)}
                    class="group flex w-full items-center gap-4 px-4 py-3 text-left transition-colors hover:bg-white/4"
                  >
                    <span class="grid size-10 shrink-0 place-items-center rounded-lg bg-white/6 text-muted group-hover:text-accent-ink">
                      <Folder class="size-5" aria-hidden="true" />
                    </span>
                    <span class="min-w-0 flex-1 truncate font-medium group-hover:text-accent-ink">{f.name}</span>
                  </button>
                </li>
              );
            }
            const ep = parseEpisode(f.name);
            const label = !ep.number ? f.name : sharedQuality() ? `Episódio ${Number(ep.number)}` : (ep.quality ?? ep.label);
            return (
              <li class="flex items-center gap-4 px-4 py-3 transition-colors hover:bg-white/4" title={f.name}>
                <span
                  class={cx(
                    "grid w-10 shrink-0 place-items-center font-display text-2xl font-semibold tabular-nums",
                    ep.number ? "text-accent-ink" : "text-faint",
                  )}
                >
                  {ep.number ?? <FileVideo class="size-5" aria-hidden="true" />}
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate font-medium">{label}</span>
                  <span class="flex gap-3 text-xs text-faint tabular-nums">
                    <span>{f.extension?.replace(".", "").toUpperCase()}</span>
                    <span>{formatBytes(f.size)}</span>
                  </span>
                </span>
                <DownloadButton path={path} name={label} />
              </li>
            );
          }}
        </For>
      </ul>
    </Show>
  );
}
