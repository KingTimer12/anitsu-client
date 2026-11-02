import { openUrl } from "@tauri-apps/plugin-opener";
import { BookOpen, Download, ExternalLink, Play, Star } from "lucide-solid";
import { formatTime, videoFiles } from "../lib/player/logic";
import { player } from "../lib/player/session";
import { playerStore } from "../lib/player/store";
import { createEffect, createMemo, createResource, createSignal, For, Match, on, Show, Switch, type JSX } from "solid-js";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { filePaths, FileList } from "../components/FileList";
import { createAlbum, looksLikeAlbum } from "../lib/music/album";
import { AlbumView } from "./AlbumView";
import { Portrait } from "../components/Portrait";
import { Cover } from "../components/Poster";
import { Button, cx } from "../components/ui";
import { anilist, lookupMedia } from "../lib/anilist/api";
import { accentVars } from "../lib/anilist/color";
import { cleanDescription, label, mediaTitle, mediaYear } from "../lib/anilist/text";
import type { FuzzyDate, MediaDetail, MediaType } from "../lib/anilist/types";
import { AnitsuError, joinPath, nuvem, type FileEntry } from "../lib/anitsu";
import { downloads } from "../lib/downloads";
import { formatNumber } from "../lib/format";
import { folderTitle, folderYear, releaseTags, splitPath } from "../lib/library";
import { episodeNumber as parseEpisodeNumber } from "../lib/player/logic";
import { router } from "../lib/router";
import { chapterLabel, comicFiles, labelText } from "../lib/reader/logic";
import { reader } from "../lib/reader/session";
import { readerStore } from "../lib/reader/store";

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

  // --- Assistir / continuar ---
  const videos = () => (filesListing() ? videoFiles(filesListing()!.files) : []);
  function play(file: FileEntry) {
    player.open({
      dir: filesPath(),
      file,
      files: filesListing()!.files,
      titlePath: props.titlePath,
      malId: detail()?.idMal ?? null,
      isAnime: props.type === "ANIME",
      animeTitle: heading(),
    });
  }
  /** Último episódio assistido deste release: continua nele ou vai para o seguinte. */
  const resume = createMemo(() => {
    const list = videos();
    let last: { i: number; at: number; watched: boolean; when: number } | null = null;
    list.forEach((f, i) => {
      const p = playerStore.progress(joinPath(filesPath(), f.name));
      if (p && (!last || p.updatedAt >= last.when)) last = { i, at: p.position, watched: p.watched, when: p.updatedAt };
    });
    const l = last as { i: number; at: number; watched: boolean } | null;
    if (!l) return null;
    if (!l.watched) return { file: list[l.i], at: l.at };
    return list[l.i + 1] ? { file: list[l.i + 1], at: 0 } : null;
  });
  const resumeLabel = (r: { file: FileEntry; at: number }) => {
    const n = parseEpisodeNumber(r.file.name);
    const ep = n !== null ? `ep. ${n}` : "";
    return r.at > 0 ? `Continuar ${ep} (${formatTime(r.at)})`.replace("  ", " ") : `Assistir ${ep}`.trim();
  };

  // --- Músicas: pasta com faixas vira encarte ---
  const [album] = createAlbum(() => {
    const l = filesListing();
    return l && looksLikeAlbum(l.files) ? { path: filesPath(), files: l.files } : undefined;
  });

  // --- Ler / continuar (mangás) ---
  const comics = () => (filesListing() ? comicFiles(filesListing()!.files) : []);
  function read(file: FileEntry) {
    reader.open({ dir: filesPath(), file, files: filesListing()!.files, titlePath: props.titlePath, title: heading() });
  }
  /** Último arquivo lido desta pasta: continua nele ou vai para o seguinte. */
  const resumeRead = createMemo(() => {
    const list = comics();
    let best: { i: number; page: number; read: boolean; when: number; unitName?: string } | null = null;
    list.forEach((f, i) => {
      const p = readerStore.progress(joinPath(filesPath(), f.name));
      if (p && (!best || p.updatedAt >= best.when))
        best = { i, page: p.page, read: p.read, when: p.updatedAt, unitName: p.unitName };
    });
    const b = best as { i: number; page: number; read: boolean; unitName?: string } | null;
    if (!b) return list[0] ? { file: list[0], page: 0, resume: false } : null;
    if (!b.read) return { file: list[b.i], page: b.page, resume: true, unitName: b.unitName };
    return list[b.i + 1] ? { file: list[b.i + 1], page: 0, resume: false } : null;
  });
  const readLabel = (r: { file: FileEntry; page: number; resume: boolean; unitName?: string }) => {
    // Coletânea: o progresso guarda o capítulo interno, não a página.
    if (r.resume && r.unitName) return `Continuar ${labelText(chapterLabel(r.unitName))}`;
    const l = labelText(chapterLabel(r.file.name));
    return r.resume ? `Continuar ${l} (pág. ${r.page + 1})` : `Ler ${l}`;
  };

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

  // --- Abas ---
  const [tab, setTab] = createSignal<TabId>("files");
  createEffect(on(() => props.titlePath, () => setTab("files")));
  const tabs = createMemo<TabInfo[]>(() => {
    const d = full();
    const list: TabInfo[] = [
      { id: "files", label: props.type === "MANGA" ? "Arquivos" : "Episódios", count: episodePaths().length || undefined },
    ];
    if (d?.characters.edges.length) list.push({ id: "cast", label: "Personagens", count: d.characters.edges.length });
    if (d?.staff.edges.length) list.push({ id: "staff", label: "Equipe", count: new Set(d.staff.edges.map((e) => e.node.id)).size });
    if (d?.relations.edges.length) list.push({ id: "related", label: "Relacionadas", count: d.relations.edges.length });
    if (d) list.push({ id: "details", label: "Detalhes" });
    return list;
  });
  const activeTab = () => (tabs().some((t) => t.id === tab()) ? tab() : "files");

  const meta = createMemo(() => {
    const m = media();
    if (!m) return [];
    return [
      m.format && label.format(m.format),
      mediaYear(m),
      m.episodes && `${m.episodes} ${m.episodes === 1 ? "episódio" : "episódios"}`,
      m.chapters && `${m.chapters} capítulos`,
      full()?.studios.nodes[0]?.name,
    ].filter(Boolean) as (string | number)[];
  });

  const singleReleaseTags = () => (releases().length === 1 ? releaseTags(releases()[0].name) : []);

  return (
    <div class="relative min-h-full" style={accent() as unknown as Record<string, string>}>
      {/* Fundo: banner esmaecido + brilho na cor da capa. */}
      <div aria-hidden="true" class="pointer-events-none absolute inset-x-0 top-0 h-[380px] overflow-hidden">
        <div class="absolute inset-0 bg-[radial-gradient(60%_100%_at_70%_0%,var(--accent-glow),transparent_70%)]" />
        <Show when={banner()}>
          <img
            src={banner()!}
            alt=""
            class={cx(
              "absolute inset-0 size-full animate-fade-in object-cover object-[center_30%]",
              media()?.bannerImage ? "opacity-45" : "scale-125 opacity-30 blur-3xl saturate-150",
            )}
          />
        </Show>
        <div class="absolute inset-0 bg-[linear-gradient(180deg,rgb(11_11_16/0.2)_0%,rgb(11_11_16/0.75)_60%,var(--color-canvas)_100%)]" />
      </div>

      <div class="relative mx-auto max-w-6xl px-8 pt-5 pb-16">
        <Breadcrumbs path={props.titlePath} class="[&_li]:text-fg/75" />

        <header class="mt-20 grid grid-cols-[184px_minmax(0,1fr)] items-start gap-9 max-lg:grid-cols-[136px_minmax(0,1fr)] max-lg:gap-6">
          <Cover
            media={media()}
            name={lookupName()}
            eager
            class="animate-fade-up shadow-2xl shadow-black/60 ring-1 ring-white/10"
          />
          <div class="min-w-0 pt-2">
            <Show when={media()?.title.native && media()!.title.native !== heading()}>
              <p lang="ja" class="mb-1 text-sm text-muted">
                {media()!.title.native}
              </p>
            </Show>
            <h1 class="font-display text-[2.75rem] leading-[1.05] font-bold tracking-tight text-balance max-lg:text-4xl">
              {heading()}
            </h1>
            <Show when={media()?.title.english && media()!.title.english !== heading()}>
              <p class="mt-1.5 text-muted">{media()!.title.english}</p>
            </Show>

            <Show when={media()}>
              {(m) => (
                <p class="mt-4 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted">
                  <Show when={m().averageScore}>
                    <span class="flex items-center gap-1 font-semibold text-fg" title="Nota média no AniList">
                      <Star class="size-3.5 fill-amber-300 text-amber-300" aria-hidden="true" />
                      {m().averageScore}%<span class="sr-only"> de nota média</span>
                    </span>
                    <Show when={meta().length}>
                      <Sep />
                    </Show>
                  </Show>
                  <For each={meta()}>
                    {(v, i) => (
                      <>
                        <Show when={i() > 0}>
                          <Sep />
                        </Show>
                        <span>{v}</span>
                      </>
                    )}
                  </For>
                </p>
              )}
            </Show>

            <Show when={media()?.genres.length}>
              <p class="mt-2 text-sm text-accent-ink">
                <span class="sr-only">Gêneros: </span>
                {media()!.genres.map((g) => label.genre(g)).join(", ")}
              </p>
            </Show>

            <Show when={full()}>{(d) => <Synopsis media={d()} />}</Show>
            <Show when={!summary.loading && titleListing() && !media()}>
              <p class="mt-5 max-w-[60ch] text-sm text-muted">
                Não achamos este título no AniList. Os arquivos continuam disponíveis abaixo.
              </p>
            </Show>

            <div class="mt-6 flex flex-wrap items-center gap-2">
              <Show when={resume() ?? (videos()[0] ? { file: videos()[0], at: 0 } : null)}>
                {(r) => (
                  <Button variant="primary" onClick={() => play(r().file)}>
                    <Play class="size-4" fill="currentColor" stroke-width={0} aria-hidden="true" />
                    {resumeLabel(r())}
                  </Button>
                )}
              </Show>
              <Show when={resumeRead()}>
                {(r) => (
                  <Button variant="primary" onClick={() => read(r().file)}>
                    <BookOpen class="size-4" aria-hidden="true" />
                    {readLabel(r())}
                  </Button>
                )}
              </Show>
              <Button
                variant={videos().length || comics().length ? "secondary" : "primary"}
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
                  <Button variant="ghost" onClick={() => openUrl(`https://www.youtube.com/watch?v=${t().id}`)}>
                    <Play class="size-4" aria-hidden="true" />
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
          </div>
        </header>

        <Tabs tabs={tabs()} active={activeTab()} onChange={setTab} />

        <div
          role="tabpanel"
          id={`panel-${activeTab()}`}
          aria-labelledby={`tab-${activeTab()}`}
          class="pt-8"
        >
          <Switch>
            <Match when={activeTab() === "files"}>
              <div class="flex flex-col gap-5">
                <Show when={seasons().length > 1 || releases().length > 1 || singleReleaseTags().length}>
                  <div class="flex flex-col gap-3">
                    <Show when={seasons().length > 1}>
                      <ChipGroup label="Temporada">
                        <For each={seasons()}>
                          {(s) => (
                            <Chip
                              active={s.name === seasonName()}
                              title={s.name}
                              onClick={() => router.go(joinPath(props.titlePath, s.name))}
                            >
                              <span class="font-semibold tabular-nums">{folderYear(s.name)}</span>
                              <span class="max-w-56 truncate">{folderTitle(s.name)}</span>
                            </Chip>
                          )}
                        </For>
                      </ChipGroup>
                    </Show>
                    <Show when={releases().length > 1}>
                      <ChipGroup label="Versão">
                        <For each={releases()}>
                          {(r) => {
                            const tags = releaseTags(r.name);
                            return (
                              <Chip
                                active={r.name === releaseName()}
                                title={r.name}
                                onClick={() => router.go(joinPath(seasonPath(), r.name))}
                              >
                                {tags.length ? tags.join(" ") : folderTitle(r.name)}
                              </Chip>
                            );
                          }}
                        </For>
                      </ChipGroup>
                    </Show>
                    <Show when={singleReleaseTags().length}>
                      <ChipGroup label="Versão">
                        <span class="pt-1.5 text-sm text-muted" title={releases()[0]?.name}>
                          {singleReleaseTags().join(" ")}
                        </span>
                      </ChipGroup>
                    </Show>
                  </div>
                </Show>

                <Show when={filesListing.error}>
                  <p class="text-danger">{AnitsuError.from(filesListing.error).message}</p>
                </Show>
                <Show when={filesListing()} fallback={<div class="skeleton h-80 rounded-2xl" />}>
                  <Show
                    when={looksLikeAlbum(filesListing()!.files) && !(album.state === "ready" && !album()) && !album.error}
                    fallback={<FileList dir={filesPath()} files={filesListing()!.files} onOpenFolder={router.go} onPlay={play} onRead={read} />}
                  >
                    <Show when={album()} fallback={<div class="skeleton h-80 rounded-2xl" />}>
                      {(d) => <AlbumView data={d()} work={heading()} />}
                    </Show>
                  </Show>
                </Show>
              </div>
            </Match>
            <Match when={activeTab() === "cast" && full()}>{(d) => <Cast media={d()} />}</Match>
            <Match when={activeTab() === "staff" && full()}>{(d) => <Staff media={d()} />}</Match>
            <Match when={activeTab() === "related" && full()}>{(d) => <Relations media={d()} />}</Match>
            <Match when={activeTab() === "details" && full()}>{(d) => <Facts media={d()} />}</Match>
          </Switch>
        </div>
      </div>
    </div>
  );
}

type TabId = "files" | "cast" | "staff" | "related" | "details";
type TabInfo = { id: TabId; label: string; count?: number };

const Sep = () => <span aria-hidden="true" class="size-1 rounded-full bg-faint/60" />;

/** Abas com sublinhado. Setas/Home/End trocam de aba (padrão WAI-ARIA). */
function Tabs(props: { tabs: TabInfo[]; active: TabId; onChange: (id: TabId) => void }) {
  let list!: HTMLDivElement;
  function onKey(e: KeyboardEvent) {
    if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(e.key)) return;
    e.preventDefault();
    const n = props.tabs.length;
    const i = props.tabs.findIndex((t) => t.id === props.active);
    const next = e.key === "Home" ? 0 : e.key === "End" ? n - 1 : (i + (e.key === "ArrowRight" ? 1 : -1) + n) % n;
    props.onChange(props.tabs[next].id);
    list.querySelectorAll<HTMLButtonElement>("[role=tab]")[next]?.focus();
  }
  return (
    <div
      ref={list}
      role="tablist"
      aria-label="Seções do título"
      onKeyDown={onKey}
      class="scrollbar-none sticky top-0 z-20 -mx-8 mt-12 flex gap-1 overflow-x-auto border-b border-line bg-canvas/85 px-8 backdrop-blur-xl"
    >
      <For each={props.tabs}>
        {(t) => {
          const active = () => t.id === props.active;
          return (
            <button
              type="button"
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={active()}
              aria-controls={`panel-${t.id}`}
              tabindex={active() ? 0 : -1}
              onClick={() => props.onChange(t.id)}
              class={cx(
                "relative flex h-12 shrink-0 items-center gap-2 px-3 text-sm font-medium transition-colors duration-150",
                active() ? "text-fg" : "text-muted hover:text-fg",
              )}
            >
              {t.label}
              <Show when={t.count}>
                <span class={cx("text-xs tabular-nums", active() ? "text-accent-ink" : "text-faint")}>{t.count}</span>
              </Show>
              <span
                aria-hidden="true"
                class={cx(
                  "absolute inset-x-3 -bottom-px h-0.5 rounded-full bg-accent transition-opacity duration-200",
                  active() ? "opacity-100" : "opacity-0",
                )}
              />
            </button>
          );
        }}
      </For>
    </div>
  );
}

function ChipGroup(props: { label: string; children: JSX.Element }) {
  return (
    <div role="group" aria-label={props.label} class="flex items-start gap-4">
      <span class="w-20 shrink-0 pt-1.5 text-xs text-faint">{props.label}</span>
      <div class="flex min-w-0 flex-wrap gap-2">{props.children}</div>
    </div>
  );
}

function Chip(props: { active: boolean; title?: string; onClick: () => void; children: JSX.Element }) {
  return (
    <button
      type="button"
      aria-pressed={props.active}
      title={props.title}
      onClick={props.onClick}
      class={cx(
        "inline-flex h-8 items-center gap-2 rounded-full border px-3.5 text-sm transition-colors duration-150",
        props.active
          ? "border-accent/60 bg-accent-soft text-fg"
          : "border-line text-muted hover:border-line-strong hover:text-fg",
      )}
    >
      {props.children}
    </button>
  );
}

function Synopsis(props: { media: MediaDetail }) {
  const desc = () => cleanDescription(props.media.description);
  const [expanded, setExpanded] = createSignal(false);
  const long = () => desc().paragraphs.join(" ").length > 300;

  return (
    <Show when={desc().paragraphs.length}>
      <div class="mt-5 max-w-[68ch]">
        <div lang="en" class={cx("space-y-3 leading-relaxed text-fg/80 select-text", long() && !expanded() && "line-clamp-3")}>
          <For each={desc().paragraphs}>{(p) => <p>{p}</p>}</For>
        </div>
        <Show when={long() || desc().source}>
          <div class="mt-1.5 flex items-center gap-3 text-sm">
            <Show when={long()}>
              <button
                type="button"
                onClick={() => setExpanded((v) => !v)}
                aria-expanded={expanded()}
                class="font-medium text-accent-ink hover:underline"
              >
                {expanded() ? "Mostrar menos" : "Ler mais"}
              </button>
            </Show>
            <Show when={(expanded() || !long()) && desc().source}>
              <span class="text-xs text-faint">Fonte: {desc().source}</span>
            </Show>
          </div>
        </Show>
      </div>
    </Show>
  );
}

function Cast(props: { media: MediaDetail }) {
  return (
    <ul class="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-x-8 gap-y-1">
      <For each={props.media.characters.edges}>
        {(e) => {
          const va = e.voiceActors[0];
          return (
            <li class="flex items-center gap-3 border-b border-line py-3">
              <Portrait src={e.node.image?.large} name={e.node.name.full} class="size-12 shrink-0 rounded-full text-sm" />
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-semibold">{e.node.name.full}</span>
                <span class="text-xs text-faint">{label.characterRole(e.role)}</span>
              </span>
              <Show when={va}>
                <span class="min-w-0 flex-1 text-right">
                  <span class="block truncate text-sm">{va.name.full}</span>
                  <span lang="ja" class="text-xs text-faint">
                    {va.name.native}
                  </span>
                </span>
                <Portrait src={va.image?.large} name={va.name.full} class="size-12 shrink-0 rounded-full text-sm" />
              </Show>
            </li>
          );
        }}
      </For>
    </ul>
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
    <ul class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-x-8 gap-y-1">
      <For each={people()}>
        {(p) => (
          <li class="flex items-center gap-3 border-b border-line py-3">
            <Portrait src={p.node.image?.large} name={p.node.name.full} class="size-12 shrink-0 rounded-full text-sm" />
            <span class="min-w-0">
              <span class="block truncate text-sm font-semibold">{p.node.name.full}</span>
              <span class="line-clamp-2 text-xs text-muted">{p.roles.join(", ")}</span>
            </span>
          </li>
        )}
      </For>
    </ul>
  );
}

function Relations(props: { media: MediaDetail }) {
  const url = (type: string, id: number) => `https://anilist.co/${type.toLowerCase()}/${id}`;
  return (
    <>
      <p class="mb-5 text-sm text-faint">Abrem no AniList.</p>
      <ul class="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-x-5 gap-y-7">
        <For each={props.media.relations.edges}>
          {(e) => (
            <li>
              <button type="button" onClick={() => openUrl(url(e.node.type, e.node.id))} class="group w-full text-left">
                <div
                  class="mb-2 aspect-[2/3] overflow-hidden rounded-xl bg-surface ring-1 ring-white/6"
                  style={{ "background-color": e.node.coverImage?.color ?? undefined }}
                >
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
    </>
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
  const tags = () => m().tags.filter((t) => !t.isMediaSpoiler && t.rank >= 60).slice(0, 12);
  const groups = (): [string, [string, string | null | undefined][]][] => [
    [
      "Obra",
      [
        ["Formato", m().format && label.format(m().format)],
        ["Origem", m().source && label.source(m().source)],
        ["Estúdio", m().studios.nodes.map((s) => s.name).join(", ") || null],
        ["Episódios", m().episodes ? String(m().episodes) : null],
        ["Duração", m().duration ? `${m().duration} min por episódio` : null],
        ["Capítulos", m().chapters ? String(m().chapters) : null],
        ["Volumes", m().volumes ? String(m().volumes) : null],
      ],
    ],
    [
      "Exibição",
      [
        ["Situação", m().status && label.status(m().status)],
        ["Temporada", m().season && m().seasonYear ? `${label.season(m().season)} de ${m().seasonYear}` : null],
        ["Estreia", formatDate(m().startDate)],
        ["Fim", formatDate(m().endDate)],
      ],
    ],
    [
      "AniList",
      [
        ["Nota média", m().averageScore ? `${m().averageScore}%` : null],
        ["Popularidade", m().popularity ? `${formatNumber(m().popularity!)} pessoas` : null],
        ["Outros nomes", m().synonyms.slice(0, 4).join("; ") || null],
      ],
    ],
  ];

  return (
    <div class="grid gap-10">
      <div class="grid grid-cols-3 gap-10 max-lg:grid-cols-1">
        <For each={groups().map(([g, rows]) => [g, rows.filter(([, v]) => v)] as const).filter(([, rows]) => rows.length)}>
          {([g, rows]) => (
            <section aria-label={g}>
              <h3 class="mb-3 text-sm font-semibold text-fg">{g}</h3>
              <dl class="divide-y divide-line">
                <For each={rows}>
                  {([k, v]) => (
                    <div class="flex justify-between gap-6 py-2.5 text-sm">
                      <dt class="shrink-0 text-faint">{k}</dt>
                      <dd class="text-right select-text">{v}</dd>
                    </div>
                  )}
                </For>
              </dl>
            </section>
          )}
        </For>
      </div>
      <Show when={tags().length}>
        <section aria-label="Temas">
          <h3 class="mb-3 text-sm font-semibold text-fg">Temas</h3>
          <ul class="flex flex-wrap gap-1.5">
            <For each={tags()}>
              {(t) => <li class="rounded-full border border-line px-3 py-1 text-xs text-muted">{t.name}</li>}
            </For>
          </ul>
        </section>
      </Show>
    </div>
  );
}
