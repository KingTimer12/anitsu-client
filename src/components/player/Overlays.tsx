import { ChevronsRight, Play } from "lucide-solid";
import { createEffect, createSignal, on, onCleanup, Show } from "solid-js";

/** Botão "Pular abertura", no canto inferior direito; visível mesmo com os controles escondidos. */
export function SkipButton(props: { onSkip(): void }) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        props.onSkip();
      }}
      title="Pular abertura (S)"
      class="flex animate-pop items-center gap-2 rounded-xl border border-line-strong bg-black/70 px-4 py-2.5 text-sm font-semibold text-fg shadow-xl backdrop-blur-md transition hover:border-accent hover:bg-black/85 active:scale-95"
    >
      Pular abertura
      <ChevronsRight class="size-4.5 text-accent-ink" aria-hidden="true" />
    </button>
  );
}

/**
 * Cartão de próximo episódio. Com `countdown`, conta até zero e chama `onPlay`;
 * sem ele, só oferece o botão.
 */
export function NextCard(props: {
  nextLabel: string;
  countdown: number | null;
  /** Vídeo pausado: a contagem para junto. */
  paused: boolean;
  onPlay(): void;
  onCancel(): void;
}) {
  const [left, setLeft] = createSignal(props.countdown ?? 0);

  createEffect(
    on(
      () => props.countdown,
      (total) => {
        if (total === null) return;
        setLeft(total);
        let last = performance.now();
        const timer = setInterval(() => {
          const now = performance.now();
          const step = props.paused ? 0 : (now - last) / 1000;
          last = now;
          const remaining = Math.max(0, left() - step);
          setLeft(remaining);
          if (remaining <= 0) {
            clearInterval(timer);
            props.onPlay();
          }
        }, 100);
        onCleanup(() => clearInterval(timer));
      },
    ),
  );

  const fill = () => (props.countdown ? `${(1 - left() / props.countdown) * 100}%` : "0%");

  return (
    <div
      role="dialog"
      aria-label="Próximo episódio"
      onClick={(e) => e.stopPropagation()}
      class="w-80 animate-pop rounded-2xl border border-line-strong bg-surface/90 p-4 shadow-2xl backdrop-blur-xl"
    >
      <p class="text-xs text-muted" aria-live="polite">
        <Show when={props.countdown !== null} fallback="Próximo episódio">
          Próximo episódio em {Math.ceil(left())}
        </Show>
      </p>
      <p class="mt-0.5 mb-3 truncate font-display font-semibold" title={props.nextLabel}>
        {props.nextLabel}
      </p>
      <div class="flex gap-2">
        <button
          type="button"
          onClick={props.onPlay}
          class="relative flex flex-1 items-center justify-center gap-1.5 overflow-hidden rounded-lg bg-accent px-3 py-2 text-sm font-semibold text-black transition active:scale-95"
        >
          <span class="absolute inset-y-0 left-0 bg-white/30" style={{ width: fill() }} aria-hidden="true" />
          <Play class="relative size-4" fill="currentColor" stroke-width={0} aria-hidden="true" />
          <span class="relative">Assistir agora</span>
        </button>
        <button
          type="button"
          onClick={props.onCancel}
          class="rounded-lg border border-line-strong px-3 py-2 text-sm font-medium text-fg transition hover:bg-white/8 active:scale-95"
        >
          Cancelar
        </button>
      </div>
    </div>
  );
}
