import { createMemo, createResource, For, Show } from "solid-js";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { FileList } from "../components/FileList";
import { sectionMeta } from "../components/sections";
import { cx } from "../components/ui";
import { AnitsuError, joinPath, nuvem } from "../lib/anitsu";
import { splitPath } from "../lib/library";
import { router } from "../lib/router";

/** Pasta comum. Se só tiver pastas de nome curto (ex.: `Letra A`, `0-9`), vira índice em blocos. */
export function FolderView(props: { path: string }) {
  const [listing] = createResource(() => props.path, nuvem.list);
  const segs = () => splitPath(props.path);
  const title = () => (segs().length === 1 ? sectionMeta(segs()[0]).label : (segs().at(-1) ?? ""));

  const tiles = createMemo(() => {
    const files = listing()?.files ?? [];
    const dirs = files.filter((f) => f.is_directory);
    const isIndex = dirs.length >= 6 && dirs.length === files.length && dirs.every((d) => d.name.length <= 16);
    return isIndex ? dirs.map((d) => d.name).sort((a, b) => a.localeCompare(b, "pt-BR", { numeric: true })) : null;
  });

  return (
    <div class="mx-auto max-w-7xl px-8 py-6">
      <Breadcrumbs path={props.path} />
      <h1 class="mt-5 mb-8 font-display text-4xl font-semibold tracking-tight">{title()}</h1>
      <Show when={listing.error}>
        <p class="text-danger">{AnitsuError.from(listing.error).message}</p>
      </Show>
      <Show when={listing()} fallback={<div class="skeleton h-72 rounded-2xl" />}>
        <Show when={tiles()} fallback={<FileList dir={props.path} files={listing()!.files} onOpenFolder={router.go} />}>
          <ul class="grid grid-cols-[repeat(auto-fill,minmax(96px,1fr))] gap-3">
            <For each={tiles()}>
              {(name) => (
                <li>
                  <button
                    type="button"
                    onClick={() => router.go(joinPath(props.path, name))}
                    class="grid aspect-square w-full place-items-center rounded-2xl border border-line bg-surface/70 p-2 text-center font-display font-semibold transition duration-200 ease-expo hover:-translate-y-0.5 hover:border-brand/60 hover:text-brand active:scale-95"
                    title={name}
                  >
                    <span
                      class={cx(
                        "leading-tight [overflow-wrap:anywhere]",
                        name.replace(/^Letra\s+/i, "").length <= 3 ? "text-3xl" : "text-sm",
                      )}
                    >
                      {name.replace(/^Letra\s+/i, "").replace(/^\[(.*)\]$/, "$1")}
                    </span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </Show>
    </div>
  );
}
