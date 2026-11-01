import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, ExternalLink, Play, Star } from "lucide-solid";
import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { filePaths, FileList } from "../components/FileList";
import { Portrait } from "../components/Portrait";
import { Cover } from "../components/Poster";
import { Button, cx, SectionTitle } from "../components/ui";
import { anilist, lookupMedia } from "../lib/anilist/api";
import { accentVars } from "../lib/anilist/color";
import { cleanDescription, label, mediaTitle, mediaYear } from "../lib/anilist/text";
import type { FuzzyDate, MediaDetail, MediaType } from "../lib/anilist/types";
import { AnitsuError, joinPath, nuvem, type FileEntry } from "../lib/anitsu";
import { downloads } from "../lib/downloads";
import { formatNumber } from "../lib/format";
import { folderTitle, folderYear, releaseTags, splitPath } from "../lib/library";
import { router } from "../lib/router";

interface Props {
  titlePath: string;
  rest: string[];
  type: MediaType;
}

const dirs = (files: FileEntry[] | undefined) =>
  (files ?? [])
    .filter((f) => f.is_directory)
    .sort((a, b) => a.name.localeCompare(b.name, "pt-BR", { numeric: true }));

/**
 * Página de um título. Estrutura esperada na nuvem:
 *   <Título>/[ANO] <Temporada>/<Release [tags]>/arquivos
 * Temporadas = subpastas com ano; releases = subpastas da temporada.
 * Cada temporada casa com sua própria entrada no AniList.
 */
export function TitleView(props: Props) {
  const titleName = () => splitPath(props.titlePath).at(-1) ?? "";

  // --- Estrutura de pastas ---
  const [titleListing] = createResource(() => props.titlePath, nuvem.list);
  const seasons = createMemo(() => dirs(titleListing()?.files).filter((d) => folderYear(d.name) !== null));
  const seasonName = createMemo(() => {
    if (!seasons().length) return null;
    const wanted = props.rest[0];
    return seasons().some((s) => s.name === wanted) ? wanted : seasons()[0].name;
  });
  const seasonPath = () => (seasonName() ? joinPath(props.titlePath, seasonName()!) : props.titlePath);
  const restAfterSeason = () => (seasonName() && props.rest[0] === seasonName() ? props.rest.slice(1) : props.rest);

  const [seasonListing] = createResource(() => (seasonName() ? seasonPath() : undefined), nuvem.list);
  const levelFiles = () => (seasonName() ? seasonListing()?.files : titleListing()?.files);
  const releases = createMemo(() =>
    seasonName() ? dirs(levelFiles()) : dirs(levelFiles()).filter((d) => folderYear(d.name) === null),
  );
  const releaseName = createMemo(() => {
    if (!releases().length) return null;
    const wanted = restAfterSeason()[0];
    return releases().some((r) => r.name === wanted) ? wanted : releases()[0].name;
  });
  const filesPath = createMemo(() => {
    if (!releaseName()) return seasonPath();
    const deeper = restAfterSeason()[0] === releaseName() ? restAfterSeason().slice(1) : [];
    return [seasonPath(), releaseName()!, ...deeper].join("/");
  });
  const [filesListing] = createResource(filesPath, nuvem.list);
  const episodePaths = () => (filesListing() ? filePaths(filesPath(), filesListing()!.files) : []);
  const remaining = () =>
    episodePaths().filter((p) => {
      const s = downloads.byPath(p)?.status;
      return s !== "done" && s !== "running" && s !== "queued";
    });

  // --- AniList ---
  const lookupName = () => seasonName() ?? titleName();
  const [summary] = createResource(
    () => (titleListing() ? { name: lookupName(), type: props.type } : undefined),
    ({ name, type }) => lookupMedia(name, type),
  );
  const [detail] = createResource(() => summary()?.id, (id) => anilist.media(id).catch(() => null));
  const media = () => detail() ?? summary() ?? null;
  const full = () => detail() ?? null;

  const accent = () => accentVars(media()?.coverImage?.color);
  const heading = () => (media() ? mediaTitle(media()!) : folderTitle(lookupName()));
  const banner = () => media()?.bannerImage ?? media()?.coverImage?.extraLarge ?? null;

  return (
    <div class="relative min-h-full" style={accent() as unknown as Record<string, string>}>
      {/* Hero: banner sangrado + brilho na cor da capa. */}
      <div aria-hidden="true" class="pointer-events-none absolute inset-x-0 top-0 h-[440px] overflow-hidden">
        <div class="absolute inset-0 bg-[radial-gradient(70%_100%_at_75%_0%,var(--accent-glow),transparent_70%)]" />
        <Show when={banner()}>
          <img
            src={banner()!}
            alt=""
            class={cx(
              "absolute inset-0 size-full animate-fade-in object-cover object-[center_30%]",
              media()?.bannerImage ? "opacity-70" : "scale-125 opacity-40 blur-3xl saturate-150",
            )}
          />
        </Show>
        <div class="absolute inset-0 bg-[linear-gradient(180deg,rgb(11_11_16/0.1)_0%,rgb(11_11_16/0.6)_55%,var(--color-canvas)_100%),linear-gradient(90deg,rgb(11_11_16/0.75)_0%,transparent_65%)]" />
      </div>

      <div class="relative mx-auto max-w-7xl px-8 pt-5 pb-16">
        <Breadcrumbs path={props.titlePath} class="[&_li]:text-fg/75" />

        <header class="mt-32 grid grid-cols-[208px_minmax(0,1fr)] items-end gap-8 max-lg:grid-cols-[150px_minmax(0,1fr)] max-lg:mt-24">
          <Cover
            media={media()}
            name={lookupName()}
            eager
            class="animate-fade-up shadow-2xl shadow-black/70 ring-1 ring-white/10"
          />
          <div class="min-w-0 animate-fade-up pb-1 [animation-delay:60ms]">
            <Show when={media()?.title.native && media()!.title.native !== heading()}>
              <p lang="ja" class="mb-1 text-sm text-muted">
                {media()!.title.native}
              </p>
            </Show>
            <h1 class="font-display text-5xl leading-[1.02] font-bold tracking-tight text-balance drop-shadow-[0_2px_24px_rgb(0_0_0/0.5)] max-lg:text-4xl">
              {heading()}
            </h1>
            <Show when={media()?.title.english && media()!.title.english !== heading()}>
              <p class="mt-2 text-lg text-muted">{media()!.title.english}</p>
            </Show>

            <Show when={media()}>
              {(m) => (
                <div class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
                  <Show when={m().averageScore}>
                    <span class="flex items-center gap-1.5 font-semibold" title="Nota média no AniList">
                      <Star class="size-4 fill-amber-300 text-amber-300" aria-hidden="true" />
                      {m().averageScore}%<span class="sr-only"> de nota média</span>
                    </span>
                  </Show>
                  <Show when={m().format}>
                    <span>{label.format(m().format)}</span>
                  </Show>
                  <Show when={mediaYear(m())}>
                    <span>{mediaYear(m())}</span>
                  </Show>
                  <Show when={m().episodes}>
                    <span>{m().episodes} episódios</span>
                  </Show>
                  <Show when={m().chapters}>
                    <span>{m().chapters} capítulos</span>
                  </Show>
                  <Show when={full()?.studios.nodes[0]}>
                    <span class="text-muted">{full()!.studios.nodes[0].name}</span>
                  </Show>
                </div>
              )}
            </Show>

            <Show when={media()?.genres.length}>
              <ul class="mt-4 flex flex-wrap gap-1.5" aria-label="Gêneros">
                <For each={media()!.genres}>
                  {(g) => (
                    <li class="rounded-full bg-accent-soft px-3 py-1 text-xs font-semibold text-accent-ink">
                      {label.genre(g)}
                    </li>
                  )}
                </For>
              </ul>
            </Show>

            <div class="mt-6 flex flex-wrap items-center gap-2">
              <Button
                variant="primary"
                disabled={!remaining().length}
                onClick={() => downloads.startAll(remaining())}
              >
                <Download class="size-4" aria-hidden="true" />
                {remaining().length
                  ? `Baixar ${remaining().length === episodePaths().length ? "todos" : "restantes"} (${remaining().length})`
                  : episodePaths().length
                    ? "Tudo na fila ou baixado"
                    : "Nada para baixar aqui"}
              </Button>
              <Show when={full()?.trailer?.site === "youtube" && full()!.trailer}>
                {(t) => (
                  <Button onClick={() => openUrl(`https://www.youtube.com/watch?v=${t().id}`)}>
                    <Play class="size-4 fill-current" aria-hidden="true" />
                    Trailer
                  </Button>
                )}
              </Show>
              <Show when={full()?.siteUrl}>
                <Button variant="ghost" onClick={() => openUrl(full()!.siteUrl!)}>
                  <ExternalLink class="size-4" aria-hidden="true" />
                  AniList
                </Button>
              </Show>
            </div>

            <Show when={!summary.loading && titleListing() && !media()}>
              <p class="mt-4 max-w-[52ch] text-sm text-muted">
                Não achamos este título no AniList. Os arquivos continuam disponíveis abaixo.
              </p>
            </Show>
          </div>
        </header>

        <div class="mt-12 grid grid-cols-[minmax(0,1fr)_300px] items-start gap-10 max-xl:grid-cols-1">
          <div class="flex min-w-0 flex-col gap-12">
            <Show when={full()}>{(d) => <Synopsis media={d()} />}</Show>

            <section aria-labelledby="files-h">
              <SectionTitle id="files-h">Episódios e arquivos</SectionTitle>

              <Show when={seasons().length > 1}>
                <div role="tablist" aria-label="Temporadas" class="scrollbar-thin mb-4 flex gap-2 overflow-x-auto pb-1">
                  <For each={seasons()}>
                    {(s) => {
                      const active = () => s.name === seasonName();
                      return (
                        <button
                          type="button"
                          role="tab"
                          aria-selected={active()}
                          onClick={() => router.go(joinPath(props.titlePath, s.name))}
                          class={cx(
                            "flex min-w-40 max-w-64 shrink-0 flex-col items-start rounded-xl border px-4 py-2.5 text-left transition-colors",
                            active()
                              ? "border-accent bg-accent-soft"
                              : "border-line bg-surface/60 hover:border-line-strong hover:bg-elevated",
                          )}
                        >
                          <span class="font-display text-lg font-semibold text-accent-ink">{folderYear(s.name)}</span>
                          <span class="max-w-full truncate text-sm text-muted">{folderTitle(s.name)}</span>
                        </button>
                      );
                    }}
                  </For>
                </div>
              </Show>

              <Show when={releases().length > 0}>
                <div role="tablist" aria-label="Versões" class="mb-4 flex flex-wrap gap-2">
                  <For each={releases()}>
                    {(r) => {
                      const tags = releaseTags(r.name);
                      const active = () => r.name === releaseName();
                      return (
                        <button
                          type="button"
                          role="tab"
                          aria-selected={active()}
                          title={r.name}
                          disabled={releases().length === 1}
                          onClick={() => router.go(joinPath(seasonPath(), r.name))}
                          class={cx(
                            "flex flex-wrap gap-1 rounded-xl border p-1.5 transition-colors disabled:cursor-default",
                            active() ? "border-accent/70 bg-accent-soft" : "border-line hover:border-line-strong",
                          )}
                        >
                          <Show when={tags.length} fallback={<span class="px-2 py-0.5 text-sm">{folderTitle(r.name)}</span>}>
                            <For each={tags}>
                              {(t) => (
                                <span
                                  class={cx(
                                    "rounded-md px-2 py-0.5 text-xs font-semibold",
                                    active() ? "bg-black/30 text-fg" : "bg-white/6 text-muted",
                                  )}
                                >
                                  {t}
                                </span>
                              )}
                            </For>
                          </Show>
                        </button>
                      );
                    }}
                  </For>
                </div>
              </Show>

              <Show when={filesListing.error}>
                <p class="text-danger">{AnitsuError.from(filesListing.error).message}</p>
              </Show>
              <Show when={filesListing()} fallback={<div class="skeleton h-80 rounded-2xl" />}>
                <FileList dir={filesPath()} files={filesListing()!.files} onOpenFolder={router.go} />
              </Show>
            </section>

            <Show when={full()?.characters.edges.length}>
              <Cast media={full()!} />
            </Show>
            <Show when={full()?.staff.edges.length}>
              <Staff media={full()!} />
            </Show>
            <Show when={full()?.relations.edges.length}>
              <Relations media={full()!} />
            </Show>
          </div>

          <Show when={full()}>{(d) => <Facts media={d()} />}</Show>
        </div>
      </div>
    </div>
  );
}

function Synopsis(props: { media: MediaDetail }) {
  const desc = () => cleanDescription(props.media.description);
  const [expanded, setExpanded] = createSignal(false);
  const long = () => desc().paragraphs.join(" ").length > 520;

  return (
    <Show when={desc().paragraphs.length}>
      <section aria-labelledby="syn-h">
        <SectionTitle id="syn-h">Sinopse</SectionTitle>
        <div
          lang="en"
          class={cx(
            "max-w-[70ch] space-y-3 leading-7 text-fg/85 select-text",
            long() && !expanded() && "max-h-[11.5rem] overflow-hidden [mask-image:linear-gradient(180deg,#000_55%,transparent)]",
          )}
        >
          <For each={desc().paragraphs}>{(p) => <p>{p}</p>}</For>
        </div>
        <div class="mt-3 flex items-center gap-4">
          <Show when={long()}>
            <button
              type="button"
              onClick={() => setExpanded((v) => !v)}
              aria-expanded={expanded()}
              class="text-sm font-medium text-accent-ink hover:underline"
            >
              {expanded() ? "Mostrar menos" : "Ler sinopse completa"}
            </button>
          </Show>
          <Show when={desc().source}>
            <span class="text-xs text-faint">Fonte: {desc().source}</span>
          </Show>
        </div>
      </section>
    </Show>
  );
}

function Cast(props: { media: MediaDetail }) {
  return (
    <section aria-labelledby="cast-h">
      <SectionTitle id="cast-h">Personagens e dubladores</SectionTitle>
      <ul class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-2.5">
        <For each={props.media.characters.edges}>
          {(e) => {
            const va = e.voiceActors[0];
            return (
              <li class="grid grid-cols-[56px_1fr_1fr_56px] items-center gap-3 overflow-hidden rounded-xl border border-line bg-surface/60">
                <Portrait src={e.node.image?.large} name={e.node.name.full} class="h-20 w-14" />
                <span class="min-w-0">
                  <span class="block text-sm leading-tight font-semibold">{e.node.name.full}</span>
                  <span class="text-xs text-faint">{label.characterRole(e.role)}</span>
                </span>
                <Show when={va} fallback={<span />}>
                  <span class="min-w-0 text-right">
                    <span class="block text-sm leading-tight font-semibold">{va.name.full}</span>
                    <span lang="ja" class="text-xs text-faint">
                      {va.name.native}
                    </span>
                  </span>
                </Show>
                <Show when={va} fallback={<span />}>
                  <Portrait src={va.image?.large} name={va.name.full} class="h-20 w-14" />
                </Show>
              </li>
            );
          }}
        </For>
      </ul>
    </section>
  );
}

function Staff(props: { media: MediaDetail }) {
  // Agrupa papéis da mesma pessoa: "Direção, Criação original".
  const people = createMemo(() => {
    const map = new Map<number, { node: MediaDetail["staff"]["edges"][number]["node"]; roles: string[] }>();
    for (const e of props.media.staff.edges) {
      const entry = map.get(e.node.id) ?? { node: e.node, roles: [] };
      entry.roles.push(label.staffRole(e.role));
      map.set(e.node.id, entry);
    }
    return [...map.values()];
  });

  return (
    <section aria-labelledby="staff-h">
      <SectionTitle id="staff-h">Equipe</SectionTitle>
      <ul class="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-x-4 gap-y-6">
        <For each={people()}>
          {(p) => (
            <li>
              <Portrait
                src={p.node.image?.large}
                name={p.node.name.full}
                class="mb-2 aspect-[3/4] w-full rounded-xl text-3xl ring-1 ring-white/6"
              />
              <p class="text-sm leading-tight font-semibold">{p.node.name.full}</p>
              <p class="mt-0.5 text-xs leading-snug text-muted">{p.roles.join(", ")}</p>
            </li>
          )}
        </For>
      </ul>
    </section>
  );
}

function Relations(props: { media: MediaDetail }) {
  const url = (type: string, id: number) => `https://anilist.co/${type.toLowerCase()}/${id}`;
  return (
    <section aria-labelledby="rel-h">
      <SectionTitle id="rel-h" aside={<span class="text-xs text-faint">Abre no AniList</span>}>
        Obras relacionadas
      </SectionTitle>
      <ul class="scrollbar-thin -mx-2 flex gap-4 overflow-x-auto px-2 pb-3">
        <For each={props.media.relations.edges}>
          {(e) => (
            <li class="w-32 shrink-0">
              <button
                type="button"
                onClick={() => openUrl(url(e.node.type, e.node.id))}
                class="group w-full text-left"
              >
                <div class="mb-2 aspect-[2/3] overflow-hidden rounded-xl bg-surface ring-1 ring-white/6" style={{ "background-color": e.node.coverImage?.color ?? undefined }}>
                  <Show when={e.node.coverImage?.large}>
                    <img
                      src={e.node.coverImage!.large!}
                      alt=""
                      loading="lazy"
                      class="size-full object-cover transition duration-500 ease-expo group-hover:scale-105"
                    />
                  </Show>
                </div>
                <span class="block text-xs font-semibold text-accent-ink">{label.relation(e.relationType)}</span>
                <span class="line-clamp-2 text-sm leading-snug font-medium group-hover:underline">
                  {e.node.title.userPreferred}
                </span>
                <span class="text-xs text-faint">
                  {label.format(e.node.format)} {e.node.startDate?.year ?? ""}
                </span>
              </button>
            </li>
          )}
        </For>
      </ul>
    </section>
  );
}

const MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
function formatDate(d: FuzzyDate | null | undefined) {
  if (!d?.year) return null;
  if (!d.month) return String(d.year);
  return `${d.day ? `${d.day} ` : ""}${MONTHS[d.month - 1]} ${d.year}`;
}

function Facts(props: { media: MediaDetail }) {
  const m = () => props.media;
  const tags = () => m().tags.filter((t) => !t.isMediaSpoiler && t.rank >= 60).slice(0, 8);
  const rows = (): [string, string | null | undefined][] => [
    ["Formato", m().format && label.format(m().format)],
    ["Episódios", m().episodes ? String(m().episodes) : null],
    ["Duração", m().duration ? `${m().duration} min por episódio` : null],
    ["Capítulos", m().chapters ? String(m().chapters) : null],
    ["Volumes", m().volumes ? String(m().volumes) : null],
    ["Situação", m().status && label.status(m().status)],
    ["Temporada", m().season && m().seasonYear ? `${label.season(m().season)} de ${m().seasonYear}` : null],
    ["Estreia", formatDate(m().startDate)],
    ["Fim", formatDate(m().endDate)],
    ["Estúdio", m().studios.nodes.map((s) => s.name).join(", ") || null],
    ["Origem", m().source && label.source(m().source)],
    ["Popularidade", m().popularity ? `${formatNumber(m().popularity!)} pessoas` : null],
    ["Outros nomes", m().synonyms.slice(0, 3).join("; ") || null],
  ];

  return (
    <aside
      aria-label="Ficha técnica"
      class="sticky top-6 rounded-2xl border border-line bg-surface/70 p-5 backdrop-blur-xl max-xl:static"
    >
      <h2 class="mb-4 font-display text-base font-semibold">Ficha técnica</h2>
      <dl class="grid gap-3.5">
        <For each={rows().filter(([, v]) => v)}>
          {([k, v]) => (
            <div>
              <dt class="text-xs text-faint">{k}</dt>
              <dd class="text-sm select-text">{v}</dd>
            </div>
          )}
        </For>
      </dl>
      <Show when={tags().length}>
        <div class="mt-5 border-t border-line pt-4">
          <h3 class="mb-2 text-xs text-faint">Temas</h3>
          <ul class="flex flex-wrap gap-1.5">
            <For each={tags()}>
              {(t) => <li class="rounded-md bg-white/6 px-2 py-0.5 text-xs text-muted">{t.name}</li>}
            </For>
          </ul>
        </div>
      </Show>
    </aside>
  );
}
