import { createSignal } from "solid-js";
import { formatDuration } from "../../lib/music/logic";

/**
 * O "fio" (mesmo do leitor) como barra de busca: linha fina, trecho baixado
 * mais claro, trecho tocado em vermelho e um estame marcando a posição.
 */
export function Thread(props: { position: number; duration: number | null; buffered: number; onSeek: (secs: number) => void }) {
  let el!: HTMLDivElement;
  const [drag, setDrag] = createSignal<number | null>(null);
  const total = () => props.duration ?? 0;
  const frac = () => (drag() ?? (total() ? props.position / total() : 0));
  const at = (e: PointerEvent) => {
    const r = el.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  };
  const pct = (f: number) => `${Math.min(1, Math.max(0, f)) * 100}%`;

  return (
    <div
      ref={el}
      role="slider"
      tabIndex={0}
      aria-label="Posição na faixa"
      aria-valuemin={0}
      aria-valuemax={Math.round(total())}
      aria-valuenow={Math.round(props.position)}
      aria-valuetext={`${formatDuration(props.position)} de ${formatDuration(total())}`}
      class={`group relative h-5 w-full touch-none outline-none ${total() ? "cursor-pointer" : "cursor-default"}`}
      onPointerDown={(e) => {
        if (!total()) return;
        el.setPointerCapture(e.pointerId);
        setDrag(at(e));
      }}
      onPointerMove={(e) => drag() !== null && setDrag(at(e))}
      onPointerUp={(e) => {
        const f = drag();
        setDrag(null);
        if (f !== null) props.onSeek(f * total());
        el.releasePointerCapture(e.pointerId);
      }}
      onKeyDown={(e) => {
        if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          e.stopPropagation();
          props.onSeek(Math.max(0, props.position + (e.key === "ArrowRight" ? 5 : -5)));
        }
      }}
    >
      <span class="absolute inset-x-0 top-1/2 h-px bg-line-strong" />
      <span class="absolute top-1/2 left-0 h-px bg-fg/25" style={{ width: pct(props.buffered) }} />
      <span class="absolute top-1/2 left-0 h-0.5 -translate-y-1/4 bg-brand" style={{ width: pct(frac()) }} />
      <span
        class="absolute top-0.5 h-4 w-px bg-brand-ink transition-transform group-hover:scale-y-125 group-focus-visible:scale-y-125"
        style={{ left: pct(frac()) }}
      >
        <span class="absolute -top-1 left-1/2 size-1.75 -translate-x-1/2 rounded-full bg-brand-ink" />
      </span>
    </div>
  );
}
