import { FolderOpen, LoaderCircle, Search } from "lucide-solid";
import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { lookupMedia } from "../lib/anilist/api";
import { nuvem, searchKindLabel, type SearchResult } from "../lib/anitsu";
import { mediaTypeForKind } from "../lib/library";
import { router } from "../lib/router";
import { cx } from "./ui";

const DEBOUNCE_MS = 250;

const KIND_STYLE: Record<string, string> = {
  anime: "bg-rose-500/15 text-rose-300",
  manga: "bg-sky-500/15 text-sky-300",
  dorama: "bg-violet-500/15 text-violet-300",
  ost: "bg-emerald-500/15 text-emerald-300",
  desenho: "bg-amber-500/15 text-amber-300",
};

/** Miniatura do AniList para resultados de anime/mangá. */
function ResultThumb(props: { result: SearchResult; index: number }) {
  const type = () => mediaTypeForKind(props.result.kind);
  // Só os primeiros resultados buscam capa, para poupar o limite do AniList.
  const [media] = createResource(
    () => (type() && props.index < 8 ? { name: props.result.name, type: type()! } : undefined),
    ({ name, type }) => lookupMedia(name, type),
  );
  return (
    <span class="grid h-12 w-9 shrink-0 place-items-center overflow-hidden rounded-md bg-elevated text-faint">
      <Show when={media()?.coverImage?.large} fallback={<FolderOpen class="size-4" aria-hidden="true" />}>
        <img src={media()!.coverImage!.large!} alt="" class="size-full object-cover" />
      </Show>
    </span>
  );
}

export function SearchBox() {
  let input!: HTMLInputElement;
  const [text, setText] = createSignal("");
  const [query, setQuery] = createSignal("");
  const [open, setOpen] = createSignal(false);
  const [active, setActive] = createSignal(0);

  let timer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(timer));
  function onInput(value: string) {
    setText(value);
    setOpen(true);
    clearTimeout(timer);
    timer = setTimeout(() => {
      setQuery(value.trim());
      setActive(0);
    }, DEBOUNCE_MS);
  }

  const [results] = createResource(query, async (q) => (q ? (await nuvem.search(q)).results : []));

  function choose(r: SearchResult) {
    router.go(r.path);
    setOpen(false);
    input.blur();
  }

  function onKeyDown(e: KeyboardEvent) {
    const list = results() ?? [];
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => Math.min(i + 1, list.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && list[active()]) {
      choose(list[active()]);
    } else if (e.key === "Escape") {
      setOpen(false);
      input.blur();
    }
  }

  const onGlobalKey = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      input.focus();
      input.select();
    }
  };
  window.addEventListener("keydown", onGlobalKey);
  onCleanup(() => window.removeEventListener("keydown", onGlobalKey));

  const parentOf = (path: string) => path.split("/").slice(0, -1).join(" / ");

  return (
    <div
      class="relative w-full max-w-[520px]"
      onFocusOut={(e) => !e.currentTarget.contains(e.relatedTarget as Node) && setOpen(false)}
    >
      <label class="group flex h-8 items-center gap-2 rounded-lg border border-line bg-white/5 px-2.5 text-faint transition-colors focus-within:border-accent/60 focus-within:bg-white/8 focus-within:text-fg">
        <span class="sr-only">Buscar na nuvem</span>
        <Show when={results.loading} fallback={<Search class="size-4 shrink-0" aria-hidden="true" />}>
          <LoaderCircle class="size-4 shrink-0 animate-spin" aria-hidden="true" />
        </Show>
        <input
          ref={input}
          type="search"
          placeholder="Buscar animes, mangás, OSTs..."
          value={text()}
          onInput={(e) => onInput(e.currentTarget.value)}
          onFocus={() => text() && setOpen(true)}
          onKeyDown={onKeyDown}
          role="combobox"
          aria-expanded={open()}
          aria-controls="search-results"
          aria-autocomplete="list"
          class="min-w-0 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-faint"
        />
        <kbd class="hidden rounded border border-line px-1.5 font-sans text-[11px] text-faint sm:block">Ctrl K</kbd>
      </label>

      <Show when={open() && query()}>
        <div
          id="search-results"
          role="listbox"
          class="scrollbar-thin absolute top-10 left-1/2 max-h-[70vh] w-[min(600px,80vw)] -translate-x-1/2 animate-pop overflow-y-auto rounded-2xl border border-line-strong bg-elevated/95 p-1.5 shadow-2xl shadow-black/60 backdrop-blur-xl"
        >
          <Show when={!results.loading || results()} fallback={<p class="p-4 text-sm text-muted">Buscando...</p>}>
            <Show
              when={results()?.length}
              fallback={
                <p class="p-4 text-sm text-muted">
                  Nada encontrado para "{query()}". Tente o título em romaji, como "Shingeki no Kyojin".
                </p>
              }
            >
              <For each={results()}>
                {(r, i) => (
                  <button
                    type="button"
                    role="option"
                    aria-selected={i() === active()}
                    onMouseEnter={() => setActive(i())}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => choose(r)}
                    class={cx(
                      "flex w-full items-center gap-3 rounded-xl p-2 text-left transition-colors duration-100",
                      i() === active() && "bg-white/8",
                    )}
                  >
                    <ResultThumb result={r} index={i()} />
                    <span class="min-w-0 flex-1">
                      <span class="block truncate font-medium">{r.name}</span>
                      <span class="block truncate text-xs text-faint">{parentOf(r.path)}</span>
                    </span>
                    <span
                      class={cx(
                        "shrink-0 rounded-full px-2 py-0.5 text-[11px] font-semibold",
                        KIND_STYLE[r.kind] ?? "bg-white/8 text-muted",
                      )}
                    >
                      {searchKindLabel[r.kind]}
                    </span>
                  </button>
                )}
              </For>
            </Show>
          </Show>
        </div>
      </Show>
    </div>
  );
}
