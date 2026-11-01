import { Search } from "lucide-solid";
import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { PosterCard } from "../components/Poster";
import { cx } from "../components/ui";
import type { MediaType } from "../lib/anilist/types";
import { AnitsuError, joinPath, nuvem } from "../lib/anitsu";
import { splitPath } from "../lib/library";
import { router } from "../lib/router";

/** Pasta cujos filhos são títulos (ex.: `Animes/Letra B`) → grade de pôsteres. */
export function ShelfView(props: { path: string; type: MediaType }) {
  const [listing] = createResource(() => props.path, nuvem.list);
  const parentPath = () => splitPath(props.path).slice(0, -1).join("/");
  const [siblings] = createResource(parentPath, nuvem.list);
  const [filter, setFilter] = createSignal("");

  const name = () => splitPath(props.path).at(-1) ?? "";
  const titles = createMemo(() => {
    const q = filter().trim().toLowerCase();
    return (listing()?.files ?? [])
      .filter((f) => f.is_directory && (!q || f.name.toLowerCase().includes(q)))
      .sort((a, b) => a.name.localeCompare(b.name, "pt-BR", { numeric: true }));
  });

  return (
    <div class="mx-auto max-w-7xl px-8 py-6">
      <Breadcrumbs path={props.path} />

      <header class="mt-5 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 class="font-display text-4xl font-semibold tracking-tight">{name()}</h1>
          <Show when={listing()}>
            <p class="mt-1 text-sm text-muted">{titles().length} títulos</p>
          </Show>
        </div>
        <label class="flex h-10 w-72 items-center gap-2 rounded-xl border border-line bg-surface/70 px-3 text-faint focus-within:border-accent/60 focus-within:text-fg">
          <Search class="size-4 shrink-0" aria-hidden="true" />
          <span class="sr-only">Filtrar títulos em {name()}</span>
          <input
            type="search"
            placeholder={`Filtrar em ${name()}`}
            value={filter()}
            onInput={(e) => setFilter(e.currentTarget.value)}
            class="min-w-0 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-faint"
          />
        </label>
      </header>

      <Show when={siblings()}>
        <nav aria-label="Outras letras" class="sticky top-0 z-20 -mx-8 mt-6 border-b border-line bg-canvas/85 px-8 py-3 backdrop-blur-xl">
          <ul class="flex flex-wrap gap-1">
            <For each={siblings()!.files.filter((f) => f.is_directory)}>
              {(s) => {
                const active = () => s.name === name();
                return (
                  <li>
                    <button
                      type="button"
                      aria-current={active() ? "page" : undefined}
                      onClick={() => router.go(joinPath(parentPath(), s.name))}
                      class={cx(
                        "h-9 min-w-9 rounded-lg px-2.5 font-display text-sm font-semibold whitespace-nowrap transition-colors",
                        active() ? "bg-brand text-white" : "text-muted hover:bg-white/8 hover:text-fg",
                      )}
                    >
                      {s.name.replace(/^Letra\s+/i, "")}
                    </button>
                  </li>
                );
              }}
            </For>
          </ul>
        </nav>
      </Show>

      <div class="mt-8">
        <Show when={listing.error}>
          <p class="text-danger">{AnitsuError.from(listing.error).message}</p>
        </Show>
        <Show
          when={listing()}
          fallback={
            <ul class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-x-5 gap-y-8">
              <For each={Array.from({ length: 12 })}>{() => <li class="skeleton aspect-[2/3] rounded-xl" />}</For>
            </ul>
          }
        >
          <Show
            when={titles().length}
            fallback={<p class="py-16 text-center text-muted">Nenhum título com "{filter()}" em {name()}.</p>}
          >
            <ul class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-x-5 gap-y-8">
              <For each={titles()}>
                {(t) => (
                  <li>
                    <PosterCard name={t.name} type={props.type} onOpen={() => router.go(joinPath(props.path, t.name))} />
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </Show>
      </div>
    </div>
  );
}
