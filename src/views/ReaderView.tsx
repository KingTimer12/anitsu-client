import "@fontsource/zen-old-mincho/700.css";
import "@fontsource/zen-old-mincho/900.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChevronLeft, CircleAlert, Download, SlidersHorizontal } from "lucide-solid";
import { batch, createEffect, createMemo, createSignal, For, Match, on, onCleanup, onMount, Show, Switch } from "solid-js";
import { createStore } from "solid-js/store";
import { IndexPanel, OptionsSheet, PagesPanel } from "../components/reader/Panels";
import { WindowControls } from "../components/TitleBar";
import { cx } from "../components/ui";
import { AnitsuError, joinPath, type FileEntry } from "../lib/anitsu";
import { downloads } from "../lib/downloads";
import { readerApi } from "../lib/reader/api";
import {
  baseUnit,
  buildSpreads,
  chapterLabel,
  comicFiles,
  expandUnits,
  keyAction,
  labelText,
  pagesOfGroup,
  sortPages,
  spreadOf,
  stripProgress,
  type OpenResult,
  type PageInfo,
  type ReadMode,
  type Unit,
} from "../lib/reader/logic";
import type { ReadTarget } from "../lib/reader/session";
import { readerStore } from "../lib/reader/store";

const appWindow = getCurrentWindow();
const MODES: ReadMode[] = ["single", "double", "long", "wide"];
/** Páginas carregadas antes de chegar nelas (modos de página). */
const PRELOAD = 3;
/** Acima disso o fio vira só linha, sem um estame por capítulo. */
const MAX_STAMENS = 120;

type Opened = { status: "ok"; result: OpenResult } | { status: "error"; message: string };
/** Um zip aberto: o arquivo, ou o `.cbz` guardado dentro dele. Pastas internas dividem o mesmo. */
const openKey = (u: Unit) => u.path + (u.inner ? `::${u.inner}` : "");

/**
 * Leitor de mangá. Todos os controles moram nas margens (lombada à esquerda,
 * número da página à direita, fio de progresso embaixo); nada flutua sobre a página.
 */
export function ReaderView(props: { target: ReadTarget; onClose: () => void }) {
  const files = createMemo(() => comicFiles(props.target.files));
  const pathOf = (f: FileEntry) => joinPath(props.target.dir, f.name);
  // Arquivos com vários capítulos dentro (coletâneas, pastas) viram várias unidades quando abertos.
  const [expanded, setExpanded] = createStore<Record<string, Unit[]>>({});
  const chapters = createMemo(() =>
    files().flatMap((f) => {
      const path = pathOf(f);
      return expanded[path] ?? [baseUnit(path, f)];
    }),
  );
  const indexOfKey = (key: string) => chapters().findIndex((u) => u.key === key);
  const startKey = pathOf(props.target.file);

  // --- Preferências ---
  const prefs = () => readerStore.prefs(props.target.titlePath);
  const settings = () => readerStore.settings();
  const dir = () => prefs().direction;
  const mode = () => prefs().mode;
  const isStrip = () => mode() === "long" || mode() === "wide";

  // --- Capítulos e páginas ---
  const [opened, setOpened] = createStore<Record<string, Opened>>({});
  const loading = new Set<string>();
  function ensure(u: Unit | undefined) {
    if (!u) return;
    const key = openKey(u);
    if (opened[key] || loading.has(key)) return;
    loading.add(key);
    readerApi
      .open(u.path, u.file.size, u.inner)
      .then((result) =>
        batch(() => {
          const parts = !u.inner && u.group === null ? expandUnits(u.path, u.file, result) : null;
          if (parts) setExpanded(u.path, parts);
          setOpened(key, { status: "ok", result });
        }),
      )
      .catch((e) => setOpened(key, { status: "error", message: AnitsuError.from(e).message }))
      .finally(() => loading.delete(key));
  }
  const listOf = (u: Unit | undefined) => {
    const o = u ? opened[openKey(u)] : undefined;
    if (o?.status !== "ok") return undefined;
    // Arquivo inteiro que virou vários capítulos: as páginas são dos capítulos, não dele.
    if (!u!.inner && u!.group === null && expanded[u!.path]) return undefined;
    return sortPages(pagesOfGroup(o.result.pages, u!.group));
  };

  /** Capítulo atual (chave da unidade: continua válida quando um arquivo se divide). */
  const [curKey, setCurKey] = createSignal(startKey);
  const chapter = () => Math.max(0, indexOfKey(curKey()));
  /** Página no capítulo atual; `-1` = ir para a última assim que o capítulo carregar. */
  const [page, setPage] = createSignal(0);
  const unit = () => chapters()[chapter()];
  const current = () => (unit() ? opened[openKey(unit()!)] : undefined);
  const list = () => listOf(unit()) ?? [];
  const total = () => list().length;
  const label = (i: number) => chapterLabel(chapters()[i]?.name ?? "");

  /** Onde abrir um capítulo: a página salva, se ainda não terminou. */
  const resumeAt = (u: Unit | undefined) => {
    const p = u && readerStore.progress(u.key);
    return p && !p.read && !p.unit ? p.page : 0;
  };

  // Proporções (largura/altura) das páginas já vistas: reservam espaço e acham páginas largas.
  const [ratios, setRatios] = createStore<Record<string, number>>({});
  const keyOf = (u: Unit, pg: number) => `${u.key}:${pg}`;

  // --- Faixa (long/wide strip): capítulos renderizados em sequência ---
  const [strip, setStrip] = createSignal<string[]>([startKey]);
  /** Página a mostrar quando a faixa renderizar (ao abrir, trocar de modo ou pular). */
  const [pendingScroll, setPendingScroll] = createSignal<{ key: string; pg: number } | null>(null);
  let scroller: HTMLDivElement | undefined;

  function goChapter(i: number, pg: number) {
    const u = chapters()[i];
    if (!u) return;
    batch(() => {
      setCurKey(u.key);
      setPage(pg);
      setEnd(false);
      setPanel(null);
      setStrip([u.key]);
    });
    ensure(u);
    if (isStrip()) setPendingScroll({ key: u.key, pg: Math.max(0, pg) });
  }

  onMount(() => {
    const i = indexOfKey(startKey);
    goChapter(i, resumeAt(chapters()[i]));
  });

  // Arquivo que se dividiu em capítulos: troca a unidade inteira pelo capítulo certo.
  createEffect(() => {
    const key = curKey();
    const parts = expanded[key];
    if (indexOfKey(key) >= 0 || !parts?.length) return;
    const summary = readerStore.progress(key);
    let pick = parts[0];
    let pg = 0;
    if (page() === -1) {
      pick = parts[parts.length - 1];
      pg = -1;
    } else if (summary?.unit) {
      const at = Math.max(0, parts.findIndex((u) => u.key === summary.unit));
      const done = readerStore.progress(parts[at].key)?.read;
      pick = parts[done && at + 1 < parts.length ? at + 1 : at];
      pg = resumeAt(pick);
    }
    batch(() => {
      setCurKey(pick.key);
      setPage(pg);
      setStrip((s) => s.map((k) => (k === key ? pick.key : k)));
    });
    ensure(pick);
    if (isStrip()) setPendingScroll({ key: pick.key, pg: Math.max(0, pg) });
  });
  // Na faixa, um arquivo emendado que se dividiu entra pelo primeiro capítulo.
  createEffect(() => {
    const s = strip();
    const fixed = s.map((k) => (indexOfKey(k) < 0 && expanded[k]?.length ? expanded[k][0].key : k));
    if (fixed.some((k, i) => k !== s[i])) {
      setStrip(fixed);
      fixed.forEach((k) => ensure(chapters()[indexOfKey(k)]));
    }
  });

  // Página "-1" (veio do capítulo seguinte voltando): resolve para a última quando carregar.
  createEffect(() => {
    if (page() === -1 && total()) setPage(total() - 1);
  });

  // Pré-carrega páginas à frente e, perto do fim, o índice do próximo arquivo.
  createEffect(() => {
    const n = total();
    const pg = page();
    if (!n || isStrip()) return;
    for (let k = 1; k <= PRELOAD; k++) {
      const p = list()[pg + k];
      const u = unit()!;
      if (p) new Image().src = readerApi.pageUrl(u.path, u.file.size, u.inner, p.index);
    }
    if (pg >= n - PRELOAD) ensure(chapters()[chapter() + 1]);
  });

  // --- Página dupla ---
  const spreads = createMemo(() => buildSpreads(total(), (pg) => (ratios[keyOf(unit()!, pg)] ?? 0) > 1));
  const spread = () => spreads()[spreadOf(spreads(), Math.max(0, page()))] ?? [];

  // Salva onde parou (na página dupla, conta a página mais adiantada do par).
  const lastVisible = () => (mode() === "double" && page() >= 0 ? (spread().at(-1) ?? page()) : page());
  createEffect(
    on([unit, lastVisible, total], ([u, pg, n]) => {
      if (!u || !n || pg < 0) return;
      readerStore.setProgress(u.key, Math.min(pg, n - 1), n);
      // Capítulo dentro de um arquivo: o arquivo guarda em qual capítulo parou.
      const parts = expanded[u.path];
      if (parts) {
        const index = parts.findIndex((p) => p.key === u.key);
        const read = !!readerStore.progress(u.key)?.read;
        readerStore.setSummary(u.path, { index, count: parts.length, read, unit: u.key, unitName: u.name });
      }
    }),
  );

  // --- Navegação nos modos de página ---
  const [end, setEnd] = createSignal(false);
  const hasNext = () => chapter() + 1 < chapters().length;

  function next() {
    if (isStrip()) return scrollBy(1);
    if (end()) return hasNext() ? goChapter(chapter() + 1, 0) : undefined;
    const last = total() - 1;
    const target = mode() === "double" ? (spreads()[spreadOf(spreads(), page()) + 1]?.[0] ?? last + 1) : page() + 1;
    if (target <= last) return setPage(target);
    if (settings().seam && hasNext()) goChapter(chapter() + 1, 0);
    else setEnd(true);
  }
  function prev() {
    if (isStrip()) return scrollBy(-1);
    if (end()) return setEnd(false);
    const target = mode() === "double" ? (spreads()[spreadOf(spreads(), page()) - 1]?.[0] ?? -1) : page() - 1;
    if (target >= 0) return setPage(target);
    if (chapter() > 0) goChapter(chapter() - 1, -1);
  }
  function scrollBy(sign: number) {
    if (!scroller) return;
    const amount = (mode() === "long" ? scroller.clientHeight : scroller.clientWidth) * 0.85 * sign;
    if (mode() === "long") scroller.scrollBy({ top: amount, behavior: "smooth" });
    else scroller.scrollBy({ left: dir() === "rtl" ? -amount : amount, behavior: "smooth" });
  }

  // --- Faixa: página atual pelo centro da tela; emenda o próximo perto do fim ---
  let raf = 0;
  function onScroll() {
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      if (!scroller) return;
      const r = scroller.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)?.closest<HTMLElement>("[data-pg]");
      if (el) {
        const key = el.dataset.ch!;
        const pg = Number(el.dataset.pg);
        if (key !== curKey() || pg !== page()) batch(() => (setCurKey(key), setPage(pg)));
      }
      const remaining =
        mode() === "long"
          ? scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight
          : scroller.scrollWidth - Math.abs(scroller.scrollLeft) - scroller.clientWidth;
      const near = remaining < (mode() === "long" ? scroller.clientHeight : scroller.clientWidth) * 2;
      const last = indexOfKey(strip().at(-1)!);
      const nextUnit = chapters()[last + 1];
      if (near && last >= 0 && nextUnit) {
        ensure(nextUnit);
        if (settings().seam && listOf(chapters()[last]) && !strip().includes(nextUnit.key))
          setStrip((s) => [...s, nextUnit.key]);
      }
    });
  }
  onCleanup(() => cancelAnimationFrame(raf));

  // Leva a faixa até a página pedida quando ela existir no DOM.
  createEffect(() => {
    strip();
    const want = pendingScroll();
    if (!want || !isStrip() || !listOf(chapters()[indexOfKey(want.key)])) return;
    requestAnimationFrame(() => {
      const el = scroller?.querySelector<HTMLElement>(`[data-ch="${CSS.escape(want.key)}"][data-pg="${want.pg}"]`);
      if (!el) return;
      el.scrollIntoView({ block: "start", inline: "start" });
      setPendingScroll(null);
    });
  });

  function setMode(m: ReadMode) {
    readerStore.setPrefs(props.target.titlePath, { mode: m });
    const pg = Math.max(0, page());
    batch(() => {
      setStrip([curKey()]);
      setEnd(false);
    });
    if (m === "long" || m === "wide") setPendingScroll({ key: curKey(), pg });
  }
  const toggleDir = () => readerStore.setPrefs(props.target.titlePath, { direction: dir() === "rtl" ? "ltr" : "rtl" });

  // --- Painéis, tela cheia, mouse ---
  const [panel, setPanel] = createSignal<"index" | "pages" | "options" | null>(null);
  // Ao fechar um painel, devolve o foco à faixa (senão as teclas de rolagem não chegam nela).
  createEffect(on(panel, (p) => !p && scroller?.isConnected && scroller.focus({ preventScroll: true }), { defer: true }));
  const toggle = (p: "index" | "pages" | "options") => setPanel((c) => (c === p ? null : p));
  const [fullscreen, setFullscreen] = createSignal(false);
  onMount(() => void appWindow.isFullscreen().then(setFullscreen));
  async function toggleFullscreen() {
    const fs = !fullscreen();
    await appWindow.setFullscreen(fs);
    setFullscreen(fs);
  }
  onCleanup(() => {
    if (fullscreen()) void appWindow.setFullscreen(false);
  });
  const [stirred, setStirred] = createSignal(false);
  let stirTimer: ReturnType<typeof setTimeout> | undefined;
  function stir() {
    setStirred(true);
    clearTimeout(stirTimer);
    stirTimer = setTimeout(() => setStirred(false), 2500);
  }
  onCleanup(() => clearTimeout(stirTimer));
  const showProgress = () => settings().progress !== "hidden" || stirred();

  // --- Teclado ---
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.altKey || e.metaKey) return;
      const k = e.key;
      if (k !== "Escape" && (e.target as HTMLElement).tagName === "INPUT") return;
      // Com o cursor/foco num painel, as teclas são do painel (rolar a lista), não da leitura.
      if (k !== "Escape" && panel() && (e.target as HTMLElement).closest?.("[role=dialog]")) return;
      if (k === "Escape") {
        e.preventDefault();
        if (panel()) setPanel(null);
        else if (fullscreen()) void toggleFullscreen();
        else props.onClose();
        return;
      }
      const lower = k.toLowerCase();
      const act: Record<string, () => void> = {
        g: () => toggle("index"),
        d: toggleDir,
        m: () => setMode(MODES[(MODES.indexOf(mode()) + 1) % MODES.length]),
        f: () => void toggleFullscreen(),
        "[": () => goChapter(chapter() - 1, 0),
        "]": () => goChapter(chapter() + 1, 0),
      };
      if (act[lower]) {
        e.preventDefault();
        return act[lower]();
      }
      // Na faixa vertical, setas e espaço rolam normalmente.
      if (mode() === "long" && k !== "ArrowLeft" && k !== "ArrowRight") return;
      const a = keyAction(k, dir());
      if (a) {
        e.preventDefault();
        if (a === "next") next();
        else prev();
      }
    };
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  // Clique nas laterais da página vira (respeitando a direção); no meio, nada.
  function onCanvasClick(e: MouseEvent) {
    if (panel()) return setPanel(null);
    if (isStrip()) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width;
    if (x < 0.4) (dir() === "rtl" ? next : prev)();
    else if (x > 0.6) (dir() === "rtl" ? prev : next)();
  }

  // --- Progresso ---
  const fraction = () => stripProgress(chapter(), chapters().length, Math.max(0, page()), total());
  const pageText = () => {
    if (!total()) return "—";
    const s = mode() === "double" ? spread() : [Math.max(0, page())];
    return s.map((p) => String(p + 1).padStart(2, "0")).join("–");
  };

  // --- Imagem de uma página ---
  function PageImg(p: { u: Unit; pg: number; info: PageInfo; class?: string; lazy?: boolean }) {
    const [attempt, setAttempt] = createSignal(0);
    const [failed, setFailed] = createSignal(false);
    const [loaded, setLoaded] = createSignal(false);
    const src = () =>
      `${readerApi.pageUrl(p.u.path, p.u.file.size, p.u.inner, p.info.index)}${attempt() ? `?r=${attempt()}` : ""}`;
    const ratio = () => ratios[keyOf(p.u, p.pg)];
    return (
      <Show
        when={!failed()}
        fallback={
          <div class={cx("grid min-h-60 place-items-center bg-surface p-6 text-center text-sm", p.class)} style={{ "aspect-ratio": ratio() ?? 0.7 }}>
            <div class="flex flex-col items-center gap-3">
              <CircleAlert class="size-5 text-danger" aria-hidden="true" />
              Esta página não carregou.
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  batch(() => (setFailed(false), setAttempt((n) => n + 1)));
                }}
                class="border border-line-strong px-3 py-1.5 text-xs hover:border-brand-ink"
              >
                Tentar de novo
              </button>
            </div>
          </div>
        }
      >
        <img
          src={src()}
          alt={`Página ${p.pg + 1}`}
          loading={p.lazy ? "lazy" : "eager"}
          decoding="async"
          draggable={false}
          data-ch={p.u.key}
          data-pg={p.pg}
          onLoad={(e) => {
            setLoaded(true);
            const img = e.currentTarget;
            if (img.naturalHeight) setRatios(keyOf(p.u, p.pg), img.naturalWidth / img.naturalHeight);
          }}
          onError={() => setFailed(true)}
          style={{ "aspect-ratio": ratio() ?? (p.lazy ? 0.7 : undefined) }}
          class={cx(
            "block select-none",
            !loaded() && "bg-[radial-gradient(rgb(255_255_255/0.06)_1px,transparent_1.3px)] bg-[length:6px_6px] bg-surface",
            p.class,
          )}
        />
      </Show>
    );
  }

  const margin = "relative bg-[radial-gradient(rgb(255_255_255/0.06)_1px,transparent_1.3px)] bg-[length:6px_6px]";
  const marginBtn =
    "grid size-11 place-items-center border border-line bg-bg text-muted transition-colors hover:border-brand-ink hover:text-fg";

  return (
    <div class="fixed inset-0 z-50 flex flex-col bg-bg text-fg select-none" onMouseMove={stir}>
      {/* Faixa de arrastar a janela */}
      <Show when={!fullscreen()}>
        <header data-tauri-drag-region class="flex h-8 shrink-0 items-stretch justify-end border-b border-line">
          <WindowControls />
        </header>
      </Show>

      <div class="grid min-h-0 flex-1 grid-cols-[88px_1fr_88px]">
        {/* Lombada: voltar, título em escrita vertical, número do capítulo */}
        <aside class={cx(margin, "border-r border-line")}>
          <div class="absolute inset-x-0 top-5 bottom-11 flex flex-col items-center gap-5">
            <button type="button" onClick={props.onClose} aria-label="Voltar" title="Voltar (Esc)" class={marginBtn}>
              <ChevronLeft class="size-5" />
            </button>
            <p
              class="min-h-0 flex-1 overflow-hidden font-mincho text-xl font-bold tracking-[0.18em] [writing-mode:vertical-rl]"
              title={props.target.title}
            >
              {props.target.title}
            </p>
            <button
              type="button"
              onClick={() => toggle("index")}
              aria-expanded={panel() === "index"}
              aria-label={`Índice. Agora: ${labelText(label(chapter()))}`}
              title="Índice (G)"
              class="group flex flex-col items-center px-1.5 py-1"
            >
              <small class="text-[11px] tracking-[0.06em] text-muted uppercase">{label(chapter()).kind || "Arq."}</small>
              <b
                class={cx(
                  "font-mincho text-[44px] leading-none font-black tabular-nums transition-colors",
                  panel() === "index"
                    ? "text-brand"
                    : "text-transparent [-webkit-text-stroke:1.4px_var(--color-brand-ink)] group-hover:text-brand/30",
                  (label(chapter()).number ?? String(chapter() + 1)).length > 3 && "text-[30px]",
                )}
              >
                {label(chapter()).number ?? chapter() + 1}
              </b>
            </button>
          </div>
        </aside>

        {/* Página */}
        <main class="relative min-w-0 overflow-hidden" onClick={onCanvasClick} onDblClick={() => void toggleFullscreen()}>
          <Switch>
            <Match when={current()?.status === "error" && current()}>
              {(err) => (
                <div class="grid h-full place-items-center p-8" role="alert">
                  <div class="flex max-w-sm flex-col items-center gap-3 text-center">
                    <CircleAlert class="size-7 text-danger" aria-hidden="true" />
                    <p class="font-mincho text-lg font-bold">Não deu para abrir este arquivo.</p>
                    <p class="text-sm text-muted">{(err() as { message: string }).message}</p>
                    <div class="mt-2 flex gap-2">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          setOpened(openKey(unit()!), undefined!);
                          ensure(unit());
                        }}
                        class="bg-brand px-4 py-2 text-sm font-semibold text-white hover:brightness-110"
                      >
                        Tentar de novo
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          downloads.start(unit()!.path);
                        }}
                        class="inline-flex items-center gap-2 border border-line-strong px-4 py-2 text-sm hover:border-brand-ink"
                      >
                        <Download class="size-4" aria-hidden="true" /> Baixar
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </Match>
            <Match when={!total()}>
              <div class="grid h-full place-items-center" role="status" aria-label="Abrindo o arquivo">
                <div class="aspect-[0.7] h-[78%] animate-pulse bg-surface bg-[radial-gradient(rgb(255_255_255/0.06)_1px,transparent_1.3px)] bg-[length:6px_6px]" />
              </div>
            </Match>

            <Match when={mode() === "single"}>
              <div class="flex h-full items-center justify-center px-4 pt-4 pb-11">
                <Show when={list()[Math.max(0, page())]} keyed>
                  {(info) => <PageImg u={unit()!} pg={Math.max(0, page())} info={info} class="h-full max-w-full object-contain shadow-[0_30px_60px_-20px_#000]" />}
                </Show>
              </div>
            </Match>

            <Match when={mode() === "double"}>
              <div class={cx("flex h-full items-center justify-center px-4 pt-4 pb-11", dir() === "rtl" && "flex-row-reverse")}>
                <For each={spread()}>
                  {(pg) => (
                    <PageImg
                      u={unit()!}
                      pg={pg}
                      info={list()[pg]}
                      class={cx("h-full object-contain", spread().length > 1 ? "max-w-1/2" : "max-w-full")}
                    />
                  )}
                </For>
              </div>
            </Match>

            <Match when={isStrip()}>
              <div
                ref={(el) => {
                  scroller = el;
                  // Foco na faixa: Home/End/PageDown/setas rolam sem precisar clicar antes.
                  requestAnimationFrame(() => el.focus({ preventScroll: true }));
                }}
                tabIndex={-1}
                onScroll={onScroll}
                onWheel={(e) => {
                  if (mode() === "wide" && scroller && Math.abs(e.deltaY) > Math.abs(e.deltaX))
                    scroller.scrollLeft += dir() === "rtl" ? -e.deltaY : e.deltaY;
                }}
                dir={mode() === "wide" ? dir() : "ltr"}
                class={cx(
                  "scrollbar-thin h-full outline-none",
                  mode() === "long" ? "overflow-x-hidden overflow-y-auto" : "flex overflow-x-auto overflow-y-hidden",
                )}
              >
                <For each={strip()}>
                  {(key) => {
                    const ch = () => indexOfKey(key);
                    return (
                    <Show when={chapters()[ch()]}>
                    {(u) => (
                    <section
                      aria-label={labelText(chapterLabel(u().name))}
                      class={cx(mode() === "long" ? "mx-auto flex max-w-[800px] flex-col" : "flex h-full shrink-0")}
                    >
                      <For each={listOf(u()) ?? []}>
                        {(info, pg) => (
                          <PageImg
                            u={u()}
                            pg={pg()}
                            info={info}
                            lazy
                            class={mode() === "long" ? "w-full" : "h-full w-auto max-w-none"}
                          />
                        )}
                      </For>
                      <Show when={!settings().seam || ch() + 1 >= chapters().length}>
                        <ChapterEnd ch={ch()} />
                      </Show>
                      <Show when={settings().seam && ch() + 1 < chapters().length && strip().includes(chapters()[ch() + 1].key)}>
                        <div
                          class={cx(
                            "flex items-center justify-center gap-3 font-mincho text-sm text-muted",
                            mode() === "long" ? "py-10" : "w-40 shrink-0 [writing-mode:vertical-rl]",
                          )}
                        >
                          <span class="h-px w-10 bg-brand" aria-hidden="true" />
                          {labelText(label(ch() + 1))}
                        </div>
                      </Show>
                    </section>
                    )}
                    </Show>
                    );
                  }}
                </For>
              </div>
            </Match>
          </Switch>

          {/* Fim do arquivo (modos de página, sem emendar) */}
          <Show when={end() && !isStrip()}>
            <div class="absolute inset-0 grid place-items-center bg-bg/85 backdrop-blur-sm" onClick={(e) => e.stopPropagation()}>
              <ChapterEnd ch={chapter()} />
            </div>
          </Show>

          {/* Fio de progresso */}
          <Show when={settings().progress === "thread" || (settings().progress === "hidden" && stirred())}>
            <Thread
              count={chapters().length}
              current={chapter()}
              fraction={fraction()}
              rtl={dir() === "rtl"}
              read={(i) => !!readerStore.progress(chapters()[i].key)?.read}
              caption={`${labelText(label(chapter()))} · pág. ${Math.max(0, page()) + 1}`}
            />
          </Show>

          <Show when={panel() === "index"}>
            <IndexPanel
              chapters={chapters()}
              current={chapter()}
              progressOf={(u) => readerStore.progress(u.key)}
              onPick={(i) => goChapter(i, resumeAt(chapters()[i]))}
            />
          </Show>
          <Show when={panel() === "pages" && total()}>
            <PagesPanel
              total={total()}
              current={Math.max(0, page())}
              onPick={(pg) => {
                setPanel(null);
                if (isStrip()) {
                  setStrip([curKey()]);
                  setPendingScroll({ key: curKey(), pg });
                } else setPage(pg);
                setEnd(false);
              }}
            />
          </Show>
          <Show when={panel() === "options"}>
            <OptionsSheet
              mode={mode()}
              direction={dir()}
              seam={settings().seam}
              progress={settings().progress}
              onMode={setMode}
              onDirection={(d) => readerStore.setPrefs(props.target.titlePath, { direction: d })}
              onSeam={(on) => readerStore.setSettings({ seam: on })}
              onProgress={(p) => readerStore.setSettings({ progress: p })}
            />
          </Show>
        </main>

        {/* Margem direita: número da página, opções, direção */}
        <aside class={cx(margin, "border-l border-line")}>
          <div class="absolute inset-x-0 top-5 bottom-11 flex flex-col items-center gap-3.5">
            <button
              type="button"
              onClick={() => toggle("pages")}
              aria-expanded={panel() === "pages"}
              aria-label={`Página ${pageText()} de ${total()}. Ir para outra página`}
              title="Ir para a página"
              class={cx("flex flex-col items-center px-1 transition-opacity", !showProgress() && "opacity-0 focus-visible:opacity-100")}
            >
              <span class={cx("font-mincho font-black leading-none tabular-nums", pageText().length > 3 ? "text-[22px]" : "text-[40px]")}>
                {pageText()}
              </span>
              <span class="mt-1 text-xs text-muted tabular-nums">de {total() || "—"}</span>
            </button>
            <button
              type="button"
              onClick={() => toggle("options")}
              aria-expanded={panel() === "options"}
              aria-label="Opções de leitura"
              title="Opções de leitura"
              class={cx(marginBtn, panel() === "options" && "border-brand text-fg")}
            >
              <SlidersHorizontal class="size-4.5" />
            </button>
            <button
              type="button"
              onClick={toggleDir}
              aria-label={`Direção: ${dir() === "rtl" ? "direita para esquerda" : "esquerda para direita"}. Trocar`}
              title="Trocar direção (D)"
              class="mt-auto flex w-11 items-center justify-center border border-line bg-bg py-3 text-xs tracking-[0.12em] text-brand-ink transition-colors hover:border-brand-ink [writing-mode:vertical-rl]"
            >
              {dir() === "rtl" ? "DIR → ESQ" : "ESQ → DIR"}
            </button>
          </div>
        </aside>
      </div>
    </div>
  );

  /** Cartão de fim de arquivo, com o próximo (ou o fim da pasta). */
  function ChapterEnd(p: { ch: number }) {
    const nextIdx = () => p.ch + 1;
    const has = () => nextIdx() < chapters().length;
    return (
      <div
        class={cx(
          "flex flex-col items-center justify-center gap-4 p-10 text-center",
          mode() === "wide" && "h-full w-80 shrink-0",
          mode() === "long" && "pb-16",
        )}
        dir="ltr"
      >
        <span class="font-mincho text-sm tracking-[0.12em] text-muted">Fim de {labelText(label(p.ch))}</span>
        <Show
          when={has()}
          fallback={
            <>
              <p class="font-mincho text-2xl font-bold">Último arquivo da pasta.</p>
              <button type="button" onClick={props.onClose} class="border border-line-strong px-4 py-2 text-sm hover:border-brand-ink">
                Voltar ao título
              </button>
            </>
          }
        >
          <p class="font-mincho text-3xl font-black">{labelText(label(nextIdx()))}</p>
          <div class="flex gap-2">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                goChapter(nextIdx(), resumeAt(chapters()[nextIdx()]));
              }}
              class="bg-brand px-5 py-2.5 text-sm font-semibold text-white hover:brightness-110 active:scale-95"
            >
              Ler {labelText(label(nextIdx()))}
            </button>
            <Show when={end()}>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setEnd(false);
                }}
                class="border border-line-strong px-4 py-2.5 text-sm hover:border-brand-ink"
              >
                Ficar aqui
              </button>
            </Show>
          </div>
        </Show>
      </div>
    );
  }
}

/** Fio vermelho de progresso; cada capítulo é um estame (como os da flor do higanbana). */
function Thread(props: {
  count: number;
  current: number;
  fraction: number;
  rtl: boolean;
  read: (i: number) => boolean;
  caption: string;
}) {
  const side = () => (props.rtl ? "right" : "left");
  const at = (i: number) => `${((i + 0.5) / props.count) * 100}%`;
  return (
    <div class="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-bg via-bg/70 to-transparent" aria-hidden="true">
      <span class="absolute inset-x-0 bottom-3.5 border-t border-dashed border-white/20" />
      <span class="absolute bottom-[13px] h-0.5 bg-brand transition-[width] duration-300" style={{ [side()]: 0, width: `${props.fraction * 100}%` }} />
      <Show when={props.count <= MAX_STAMENS}>
        <For each={Array.from({ length: props.count }, (_, i) => i)}>
          {(i) => (
            <span
              class={cx(
                "absolute bottom-3.5 w-px",
                i === props.current ? "h-5 bg-brand-ink" : props.read(i) ? "h-3 bg-brand" : "h-3 bg-white/25",
              )}
              style={{ [side()]: at(i) }}
            >
              <span
                class={cx(
                  "absolute left-1/2 -translate-x-1/2 rounded-full",
                  i === props.current ? "-top-1.5 size-1.75 bg-brand-ink" : "-top-1 size-1.25",
                  i !== props.current && (props.read(i) ? "bg-brand" : "bg-white/35"),
                )}
              />
            </span>
          )}
        </For>
      </Show>
      <span
        class="absolute bottom-6 bg-bg/90 px-1.5 text-[11px] whitespace-nowrap text-brand-ink"
        style={{
          [side()]: `clamp(70px, ${props.fraction * 100}%, calc(100% - 70px))`,
          transform: props.rtl ? "translateX(50%)" : "translateX(-50%)",
        }}
      >
        {props.caption}
      </span>
    </div>
  );
}
