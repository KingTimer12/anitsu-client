import { ChevronRight, LoaderCircle } from "lucide-solid";
import { createEffect, createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { NuvemBadge, PosterCard } from "../components/Poster";
import { sectionMeta, sectionOrder } from "../components/sections";
import { cx, SectionTitle } from "../components/ui";
import { anilist } from "../lib/anilist/api";
import { mediaTitle } from "../lib/anilist/text";
import type { MediaSummary } from "../lib/anilist/types";
import { displayName, nuvem, sessionStore } from "../lib/anitsu";
import { checkNuvem, nuvemPath } from "../lib/catalog";
import { router } from "../lib/router";
import { toast } from "../lib/toast";

const SEASON_LABEL: Record<string, string> = { WINTER: "inverno", SPRING: "primavera", SUMMER: "verão", FALL: "outono" };

/** Temporada atual no padrão AniList (dezembro já é inverno do ano seguinte). */
function currentSeason() {
  const d = new Date();
  const m = d.getMonth() + 1;
  const season = m === 12 || m <= 2 ? "WINTER" : m <= 5 ? "SPRING" : m <= 8 ? "SUMMER" : "FALL";
  return { label: SEASON_LABEL[season], year: m === 12 ? d.getFullYear() + 1 : d.getFullYear() };
}

/** Animes da temporada (AniList, uma requisição, com cache do lado Rust). */
function useSeason() {
  const [season] = createResource(() => anilist.season().catch(() => [] as MediaSummary[]));
  const list = () => season() ?? [];
  const checked = () => list().filter((m) => nuvemPath(m.id) !== undefined).length;
  const inNuvem = () => list().filter((m) => nuvemPath(m.id)).length;
  return { list, loading: () => season.loading, checked, inNuvem };
}

/**
 * Card da temporada. Só confere no acervo quando entra na tela (lazy),
 * então abrir a Home não dispara uma checagem por anime de uma vez.
 */
function SeasonCard(props: { media: MediaSummary }) {
  let el!: HTMLLIElement;
  const [visible, setVisible] = createSignal(false);

  onMount(() => {
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setVisible(true);
          io.disconnect();
        }
      },
      { rootMargin: "200px 0px" },
    );
    io.observe(el);
    onCleanup(() => io.disconnect());
  });

  createEffect(() => {
    if (visible()) void checkNuvem(props.media);
  });

  const path = () => nuvemPath(props.media.id);

  async function open() {
    const p = await checkNuvem(props.media);
    if (p) router.go(p);
    else toast.show(`"${mediaTitle(props.media)}" ainda não está na nuvem.`);
  }

  return (
    <li ref={el}>
      <PosterCard
        name={mediaTitle(props.media)}
        media={props.media}
        onOpen={open}
        dimmed={path() === null}
        badge={path() ? <NuvemBadge /> : undefined}
      />
    </li>
  );
}

export function HomeView() {
  const [root] = createResource(() => nuvem.list(""));
  const season = useSeason();
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
            <Show when={season.list().length}>
              <span class="flex items-center gap-2 text-xs text-faint" aria-live="polite">
                <Show when={season.checked() < season.list().length}>
                  <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
                </Show>
                {season.inNuvem()} no acervo
              </span>
            </Show>
          }
        >
          Temporada de {seasonInfo.label} {seasonInfo.year}
        </SectionTitle>
        <Show
          when={!season.loading()}
          fallback={
            <ul class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-7">
              <For each={[1, 2, 3, 4, 5, 6]}>
                {() => (
                  <li>
                    <div class="skeleton aspect-[2/3] rounded-xl" />
                    <div class="skeleton mt-2.5 h-4 w-3/4 rounded" />
                  </li>
                )}
              </For>
            </ul>
          }
        >
          <Show
            when={season.list().length}
            fallback={<p class="text-sm text-muted">Não foi possível carregar a temporada agora.</p>}
          >
            <ul class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-5 gap-y-7">
              <For each={season.list()}>{(m) => <SeasonCard media={m} />}</For>
            </ul>
          </Show>
        </Show>
      </section>
    </div>
  );
}
