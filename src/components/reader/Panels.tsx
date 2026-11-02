import { createMemo, createSignal, For, onMount, Show, type JSX } from "solid-js";
import { chapterLabel, labelText, type Direction, type ProgressStyle, type ReadMode, type Unit } from "../../lib/reader/logic";
import type { ReadProgress } from "../../lib/reader/store";
import { cx } from "../ui";

/** Moldura dos painéis do leitor: cantos retos, borda de nanquim, sombra funda. */
function Panel(props: { label: string; class?: string; children: JSX.Element }) {
  return (
    <div
      role="dialog"
      aria-label={props.label}
      onClick={(e) => e.stopPropagation()}
      // A roda do mouse sobre o painel rola o painel, nunca o capítulo atrás dele.
      onWheel={(e) => e.stopPropagation()}
      class={cx("absolute z-30 animate-pop overscroll-contain border border-line-strong bg-surface shadow-[0_24px_48px_-12px_rgb(0_0_0/0.8)]", props.class)}
    >
      {props.children}
    </div>
  );
}

/** Carimbo de "lido" (hanko). */
export function Hanko() {
  return (
    <span
      class="grid size-6.5 shrink-0 -rotate-12 place-items-center rounded-full border-[1.5px] border-brand font-mincho text-[13px] font-black text-brand"
      title="Lido"
    >
      <span aria-hidden="true">済</span>
      <span class="sr-only">Lido</span>
    </span>
  );
}

/** Índice: sumário dos capítulos (ou volumes) da pasta, com busca e carimbo nos lidos. */
export function IndexPanel(props: {
  chapters: Unit[];
  current: number;
  progressOf: (u: Unit) => ReadProgress | undefined;
  onPick: (i: number) => void;
}) {
  const [q, setQ] = createSignal("");
  let list!: HTMLOListElement;
  const items = createMemo(() =>
    props.chapters
      .map((u, i) => ({ u, i, l: chapterLabel(u.name) }))
      .filter(({ u, l }) => {
        const s = q().trim().toLowerCase();
        return !s || labelText(l).toLowerCase().includes(s) || u.name.toLowerCase().includes(s);
      }),
  );
  onMount(() => list.querySelector<HTMLElement>("[aria-current]")?.scrollIntoView({ block: "center" }));
  /** Tipo comum a todos ("Cap." ou "Vol."): vai uma vez no topo, não em cada linha. */
  const common = createMemo(() => {
    // Itens sem número (coletânea ainda não aberta, oneshot) não contam.
    const labels = props.chapters.map((u) => chapterLabel(u.name)).filter((l) => l.number !== null);
    const kinds = new Set(labels.map((l) => l.kind));
    return kinds.size === 1 ? [...kinds][0] : null;
  });
  const summary = () => {
    const n = props.chapters.length;
    const word = common() === "Vol." ? (n === 1 ? "volume" : "volumes") : common() === "Cap." ? (n === 1 ? "capítulo" : "capítulos") : n === 1 ? "arquivo" : "arquivos";
    return `${n} ${word}`;
  };

  return (
    <Panel label="Índice" class="bottom-12 left-24 flex max-h-[min(70vh,520px)] w-[300px] flex-col pt-4 pb-2">
      <div class="mx-4 mb-2 flex items-baseline justify-between">
        <h2 class="font-mincho text-[15px] font-bold tracking-[0.08em]">Índice</h2>
        <span class="text-xs text-muted tabular-nums">{summary()}</span>
      </div>
      <input
        type="search"
        autofocus
        value={q()}
        onInput={(e) => setQ(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && items()[0]) props.onPick(items()[0].i);
          e.stopPropagation();
        }}
        placeholder="Buscar capítulo ou volume"
        aria-label="Buscar capítulo ou volume"
        class="mx-4 mb-2 border border-line bg-bg px-3 py-2 text-sm outline-none placeholder:text-faint focus:border-brand-ink"
      />
      <ol ref={list} class="scrollbar-thin min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <For each={items()} fallback={<li class="px-4 py-6 text-center text-sm text-muted">Nada com esse nome.</li>}>
          {({ u, i, l }) => {
            const p = () => props.progressOf(u);
            const here = () => i === props.current;
            return (
              <li>
                <button
                  type="button"
                  onClick={() => props.onPick(i)}
                  aria-current={here() ? "true" : undefined}
                  title={u.name}
                  class={cx(
                    "flex w-full items-center gap-3 px-4 py-1.5 text-left transition-colors hover:bg-white/5",
                    here() && "bg-white/6",
                  )}
                >
                  <Show
                    when={l.number !== null}
                    fallback={<span class="min-w-0 flex-1 truncate text-sm">{l.name}</span>}
                  >
                    <span
                      class={cx(
                        "min-w-0 flex-1 truncate font-mincho text-lg leading-none font-bold tabular-nums",
                        here() ? "text-brand-ink" : p()?.read ? "text-muted" : "text-fg",
                      )}
                    >
                      <Show when={!common() && l.kind}>
                        <span class="mr-1.5 font-sans text-xs font-normal text-muted">{l.kind}</span>
                      </Show>
                      {l.number}
                    </span>
                  </Show>
                  <Show
                    when={p()?.read}
                    fallback={
                      <span class="shrink-0 text-xs text-muted tabular-nums">
                        {p() && !p()!.unit ? `${p()!.page + 1}/${p()!.total}` : ""}
                      </span>
                    }
                  >
                    <Hanko />
                  </Show>
                </button>
              </li>
            );
          }}
        </For>
      </ol>
    </Panel>
  );
}

/** Pular para uma página do capítulo atual. */
export function PagesPanel(props: { total: number; current: number; onPick: (page: number) => void }) {
  let grid!: HTMLDivElement;
  onMount(() => grid.querySelector<HTMLElement>("[aria-current]")?.scrollIntoView({ block: "center" }));
  return (
    <Panel label="Páginas" class="top-4 right-24 w-[300px] p-4">
      <h2 class="mb-3 font-mincho text-[15px] font-bold tracking-[0.08em]">Ir para a página</h2>
      <div ref={grid} class="scrollbar-thin grid max-h-[50vh] grid-cols-6 gap-1.5 overflow-y-auto overscroll-contain pr-1">
        <For each={Array.from({ length: props.total }, (_, i) => i)}>
          {(i) => (
            <button
              type="button"
              onClick={() => props.onPick(i)}
              aria-current={i === props.current ? "true" : undefined}
              aria-label={`Página ${i + 1}`}
              class={cx(
                "h-9 border font-mincho text-sm font-bold tabular-nums transition-colors",
                i === props.current ? "border-brand bg-brand text-white" : "border-line hover:border-brand-ink",
              )}
            >
              {i + 1}
            </button>
          )}
        </For>
      </div>
    </Panel>
  );
}

const MODES: { id: ReadMode; label: string; glyph: JSX.Element }[] = [
  { id: "single", label: "Página única", glyph: <rect x="9" y="3" width="22" height="40" /> },
  {
    id: "double",
    label: "Página dupla",
    glyph: (
      <>
        <rect x="2" y="3" width="17" height="40" />
        <rect x="21" y="3" width="17" height="40" />
      </>
    ),
  },
  {
    id: "long",
    label: "Long strip",
    glyph: (
      <>
        <rect x="12" y="-8" width="16" height="22" />
        <rect x="12" y="16" width="16" height="22" />
        <rect x="12" y="40" width="16" height="22" />
      </>
    ),
  },
  {
    id: "wide",
    label: "Wide strip",
    glyph: (
      <>
        <rect x="-6" y="10" width="14" height="26" />
        <rect x="10" y="10" width="14" height="26" />
        <rect x="26" y="10" width="14" height="26" />
      </>
    ),
  },
];

const PROGRESS: { id: ProgressStyle; label: string; hint: string }[] = [
  { id: "thread", label: "Fio", hint: "Linha vermelha com um estame em cada capítulo" },
  { id: "number", label: "Só o número", hint: "Número da página na margem, sem linha" },
  { id: "hidden", label: "Escondido", hint: "Leitura limpa. Aparece ao mover o mouse." },
];

const screentone = "bg-[radial-gradient(rgb(232_40_60/0.25)_1px,transparent_1.4px)] bg-[length:5px_5px]";

/** Opções de leitura: modo, direção, emendar capítulos e progresso. */
export function OptionsSheet(props: {
  mode: ReadMode;
  direction: Direction;
  seam: boolean;
  progress: ProgressStyle;
  onMode: (m: ReadMode) => void;
  onDirection: (d: Direction) => void;
  onSeam: (on: boolean) => void;
  onProgress: (p: ProgressStyle) => void;
}) {
  const heading = "mb-3 border-b border-line pb-2 font-mincho text-[15px] font-bold tracking-[0.06em]";
  return (
    <Panel label="Opções de leitura" class="top-4 right-24 grid w-[min(860px,calc(100vw-220px))] grid-cols-[1.4fr_1fr_1fr] gap-7 p-6 max-lg:grid-cols-1">
      <div>
        <h2 class={heading}>Modo</h2>
        <div class="grid grid-cols-4 gap-2" role="radiogroup" aria-label="Modo de leitura">
          <For each={MODES}>
            {(m) => (
              <button
                type="button"
                role="radio"
                aria-checked={props.mode === m.id}
                onClick={() => props.onMode(m.id)}
                class={cx(
                  "flex flex-col items-center gap-2.5 border px-1.5 pt-3 pb-2.5 text-xs transition-colors",
                  props.mode === m.id ? cx("border-brand text-fg", screentone) : "border-line text-muted hover:border-white/30 hover:text-fg",
                )}
              >
                <svg
                  viewBox="0 0 40 46"
                  class="h-11 overflow-hidden"
                  fill="none"
                  stroke={props.mode === m.id ? "var(--color-brand-ink)" : "rgb(255 255 255 / 0.45)"}
                  stroke-width="1.5"
                  aria-hidden="true"
                >
                  {m.glyph}
                </svg>
                {m.label}
              </button>
            )}
          </For>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={props.seam}
          onClick={() => props.onSeam(!props.seam)}
          class="mt-5 flex w-full items-center gap-3 text-left"
        >
          <span class="flex items-center" aria-hidden="true">
            <i class="h-6.5 w-5 border border-white/35" />
            <i class={cx("h-0.5 w-5.5", props.seam ? "bg-brand" : "border-t border-dashed border-white/30")} />
            <i class="h-6.5 w-5 border border-white/35" />
          </span>
          <span class="text-sm">
            Emendar capítulos
            <small class="block text-xs text-muted">Ao terminar, o próximo vem logo em seguida, sem sair do leitor.</small>
          </span>
          <span class={cx("relative ml-auto h-5.5 w-9.5 shrink-0 transition-colors", props.seam ? "bg-brand" : "bg-white/15")}>
            <span class={cx("absolute top-0.75 size-4 bg-white transition-transform", props.seam ? "translate-x-4.75" : "translate-x-0.75")} />
          </span>
        </button>
      </div>

      <div>
        <h2 class={heading}>Direção</h2>
        <div class="flex border border-line" role="radiogroup" aria-label="Direção de leitura">
          <For each={[["rtl", "Direita → esquerda"], ["ltr", "Esquerda → direita"]] as const}>
            {([id, label]) => (
              <button
                type="button"
                role="radio"
                aria-checked={props.direction === id}
                onClick={() => props.onDirection(id)}
                class={cx(
                  "flex-1 px-2 py-3 text-center text-[13px] transition-colors",
                  props.direction === id ? "bg-brand font-semibold text-white" : "text-muted hover:text-fg",
                )}
              >
                {label}
              </button>
            )}
          </For>
        </div>
        <p class="mt-2 text-xs leading-relaxed text-muted">Lembrada para este título. Títulos novos começam da direita para a esquerda.</p>
      </div>

      <div>
        <h2 class={heading}>Progresso</h2>
        <div class="flex flex-col gap-2" role="radiogroup" aria-label="Como mostrar o progresso">
          <For each={PROGRESS}>
            {(p) => (
              <button
                type="button"
                role="radio"
                aria-checked={props.progress === p.id}
                onClick={() => props.onProgress(p.id)}
                class={cx(
                  "flex items-center gap-3 border px-3 py-2.5 text-left transition-colors",
                  props.progress === p.id ? "border-brand" : "border-line hover:border-white/30",
                )}
              >
                <span class="text-[13px]">
                  {p.label}
                  <small class="block text-xs text-muted">{p.hint}</small>
                </span>
                <span
                  class={cx(
                    "ml-auto size-3.5 shrink-0 rounded-full border-[1.5px]",
                    props.progress === p.id ? "border-brand bg-[radial-gradient(var(--color-brand)_45%,transparent_50%)]" : "border-white/35",
                  )}
                  aria-hidden="true"
                />
              </button>
            )}
          </For>
        </div>
      </div>

      <div class="col-span-full flex flex-wrap gap-x-5 gap-y-2 border-t border-line pt-3.5 text-xs text-muted">
        <For
          each={[
            [["←", "→"], "virar a página"],
            [["[", "]"], "capítulo anterior / próximo"],
            [["G"], "índice"],
            [["D"], "trocar direção"],
            [["M"], "trocar modo"],
            [["F"], "tela cheia"],
            [["Esc"], "fechar"],
          ] as const}
        >
          {([keys, what]) => (
            <span>
              <For each={keys}>{(k) => <kbd class="mr-1 border border-white/20 px-1.5 font-sans text-fg">{k}</kbd>}</For>
              {what}
            </span>
          )}
        </For>
      </div>
    </Panel>
  );
}
