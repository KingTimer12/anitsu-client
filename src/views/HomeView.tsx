import { ChevronRight, LoaderCircle } from "lucide-solid";
import { createEffect, createResource, createSignal, For, Show } from "solid-js";
import { PosterCard } from "../components/Poster";
import { sectionMeta, sectionOrder } from "../components/sections";
import { cx, SectionTitle } from "../components/ui";
import { anilist } from "../lib/anilist/api";
import { mediaTitle } from "../lib/anilist/text";
import type { MediaSummary } from "../lib/anilist/types";
import { displayName, nuvem, sessionStore } from "../lib/anitsu";
import { filterInNuvem } from "../lib/catalog";
import { router } from "../lib/router";

const SEASON_LABEL: Record<string, string> = { WINTER: "inverno", SPRING: "primavera", SUMMER: "verão", FALL: "outono" };

/** Temporada atual no padrão AniList (dezembro já é inverno do ano seguinte). */
function currentSeason() {
  const d = new Date();
  const m = d.getMonth() + 1;
  const season = m === 12 || m <= 2 ? "WINTER" : m <= 5 ? "SPRING" : m <= 8 ? "SUMMER" : "FALL";
  return { label: SEASON_LABEL[season], year: m === 12 ? d.getFullYear() + 1 : d.getFullYear() };
}

/** Animes da temporada que existem no acervo, na ordem de popularidade. */
function useSeasonInNuvem() {
  const [season] = createResource(() => anilist.season().catch(() => [] as MediaSummary[]));
  const [found, setFound] = createSignal<{ media: MediaSummary; path: string; index: number }[]>([]);
  const [checking, setChecking] = createSignal(true);

  createEffect(() => {
    const list = season();
    if (!list) return;
    setFound([]);
    setChecking(true);
    void filterInNuvem(list, (media, path, index) =>
      setFound((prev) => [...prev, { media, path, index }].sort((a, b) => a.index - b.index)),
    ).finally(() => setChecking(false));
  });

  return { found, checking: () => season.loading || checking(), total: () => season()?.length ?? 0 };
}

export function HomeView() {
  const [root] = createResource(() => nuvem.list(""));
  const season = useSeasonInNuvem();
  const seasonInfo = currentSeason();
  const name = () => displayName(sessionStore.session()?.user);
  const sections = () =>
    (root()?.files ?? [])
      .filter((f) => f.is_directory)
      .map((f) => f.name)
      .sort(sectionOrder);

  return (
    <div class="mx-auto max-w-7xl px-8 py-10">
      <header class="animate-fade-up">
        <h1 class="font-display text-4xl font-semibold tracking-tight">
          <Show when={name()} fallback="Bem-vindo de volta">
            Olá, {name()}
          </Show>
        </h1>
        <p class="mt-2 text-muted">O que vamos assistir hoje? Busque com Ctrl K ou escolha uma seção.</p>
      </header>

      <section aria-labelledby="sections-h" class="mt-10">
        <h2 id="sections-h" class="sr-only">
          Seções
        </h2>
        <Show when={root.error}>
          <p class="text-danger">Não foi possível abrir a nuvem. Verifique sua conexão e reabra o app.</p>
        </Show>
        <ul class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3">
          <Show when={root()} fallback={<For each={[1, 2, 3, 4, 5, 6]}>{() => <li class="skeleton h-28 rounded-2xl" />}</For>}>
            <For each={sections()}>
              {(s) => {
                const meta = sectionMeta(s);
                return (
                  <li>
                    <button
                      type="button"
                      onClick={() => router.go(s)}
                      class="group flex h-28 w-full items-center gap-4 rounded-2xl border border-line bg-surface/70 p-5 text-left transition duration-200 ease-expo hover:-translate-y-0.5 hover:border-line-strong hover:bg-elevated active:scale-[0.98]"
                    >
                      <span class={cx("grid size-12 shrink-0 place-items-center rounded-xl", meta.tint)}>
                        <meta.icon class="size-6" />
                      </span>
                      <span class="min-w-0 flex-1">
                        <span class="block font-display text-lg font-semibold">{meta.label}</span>
                        <span class="line-clamp-2 text-sm text-muted">{meta.blurb}</span>
                      </span>
                      <ChevronRight
                        class="size-5 shrink-0 text-faint transition-transform group-hover:translate-x-0.5 group-hover:text-fg"
                        aria-hidden="true"
                      />
                    </button>
                  </li>
                );
              }}
            </For>
          </Show>
        </ul>
      </section>

      <section aria-labelledby="season-h" class="mt-14">
        <SectionTitle
          id="season-h"
          aside={
            <span class="flex items-center gap-2 text-xs text-faint" aria-live="polite">
              <Show when={season.checking()}>
                <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
              </Show>
              <Show when={season.checking()} fallback={`${season.found().length} de ${season.total()} no acervo`}>
                Procurando no acervo
              </Show>
            </span>
          }
        >
          Temporada de {seasonInfo.label} {seasonInfo.year}
        </SectionTitle>
        <Show
          when={season.found().length || season.checking()}
          fallback={<p class="text-sm text-muted">Nenhum anime desta temporada chegou ao acervo ainda.</p>}
        >
          <ul class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-7">
            <For each={season.found()}>
              {(f) => (
                <li class="animate-fade-in">
                  <PosterCard name={mediaTitle(f.media)} media={f.media} onOpen={() => router.go(f.path)} />
                </li>
              )}
            </For>
            <Show when={season.checking()}>
              <For each={Array.from({ length: Math.max(0, 6 - season.found().length) })}>
                {() => (
                  <li>
                    <div class="skeleton aspect-[2/3] rounded-xl" />
                    <div class="skeleton mt-2.5 h-4 w-3/4 rounded" />
                  </li>
                )}
              </For>
            </Show>
          </ul>
        </Show>
      </section>
    </div>
  );
}
