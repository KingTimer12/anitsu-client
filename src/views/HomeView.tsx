import { ChevronRight } from "lucide-solid";
import { createResource, For, Show } from "solid-js";
import { PosterCard } from "../components/Poster";
import { sectionMeta, sectionOrder } from "../components/sections";
import { cx, SectionTitle } from "../components/ui";
import { anilist } from "../lib/anilist/api";
import { mediaTitle } from "../lib/anilist/text";
import type { MediaSummary } from "../lib/anilist/types";
import { displayName, nuvem, sessionStore } from "../lib/anitsu";
import { router } from "../lib/router";
import { toast } from "../lib/toast";

/** Procura um anime em alta na nuvem e abre a pasta dele, se existir. */
async function openFromAniList(m: MediaSummary) {
  const candidates = [m.title.romaji, m.title.english, mediaTitle(m)].filter(Boolean) as string[];
  for (const q of candidates) {
    const { results } = await nuvem.search(q).catch(() => ({ results: [] as { kind: string; path: string }[] }));
    const hit = results.find((r) => r.kind === "anime");
    if (hit) return router.go(hit.path);
  }
  toast.show(`"${mediaTitle(m)}" ainda não está na nuvem.`);
}

export function HomeView() {
  const [root] = createResource(() => nuvem.list(""));
  const [trending] = createResource(() => anilist.trending().catch(() => []));
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

      <Show when={trending()?.length}>
        <section aria-labelledby="trending-h" class="mt-14">
          <SectionTitle id="trending-h" aside={<span class="text-xs text-faint">Dados do AniList</span>}>
            Em alta agora
          </SectionTitle>
          <p class="-mt-2 mb-5 text-sm text-muted">Clique para procurar na nuvem.</p>
          <ul class="scrollbar-thin -mx-2 flex snap-x gap-5 overflow-x-auto px-2 pb-4">
            <For each={trending()!.slice(0, 20)}>
              {(m) => (
                <li class="w-40 shrink-0 snap-start">
                  <PosterCard name={mediaTitle(m)} media={m} onOpen={() => openFromAniList(m)} />
                </li>
              )}
            </For>
          </ul>
        </section>
      </Show>
    </div>
  );
}
