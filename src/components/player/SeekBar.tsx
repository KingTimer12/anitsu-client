import { createSignal, For, Show } from "solid-js";
import { formatTime } from "../../lib/player/logic";

/**
 * Barra de progresso: carregado na memória (branco), assistido (accent) e
 * marcas de capítulo. Arrastar só move a interface; o seek vai no soltar.
 */
export function SeekBar(props: {
  position: number;
  duration: number;
  /** Até onde já está na memória (segundos absolutos). */
  buffered: number;
  chapters: number[];
  onSeek(sec: number): void;
}) {
  let track!: HTMLDivElement;
  const [drag, setDrag] = createSignal<number | null>(null);
  const [hover, setHover] = createSignal<{ x: number; t: number } | null>(null);

  const dur = () => Math.max(props.duration, 0.001);
  const pct = (t: number) => `${Math.min(100, Math.max(0, (t / dur()) * 100))}%`;
  const shown = () => drag() ?? props.position;

  const timeAt = (clientX: number) => {
    const r = track.getBoundingClientRect();
    const f = Math.min(1, Math.max(0, (clientX - r.left) / r.width));
    return { x: f * r.width, t: f * props.duration };
  };

  function down(e: PointerEvent) {
    if (!props.duration) return;
    track.setPointerCapture(e.pointerId);
    setDrag(timeAt(e.clientX).t);
  }
  function move(e: PointerEvent) {
    const at = timeAt(e.clientX);
    setHover(at);
    if (drag() !== null) setDrag(at.t);
  }
  function up() {
    const t = drag();
    setDrag(null);
    if (t !== null) props.onSeek(t);
  }
  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 30 : 5;
    if (e.key === "ArrowRight") props.onSeek(props.position + step);
    else if (e.key === "ArrowLeft") props.onSeek(props.position - step);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  return (
    <div
      ref={track}
      role="slider"
      tabindex="0"
      aria-label="Posição do vídeo"
      aria-valuemin={0}
      aria-valuemax={Math.round(props.duration)}
      aria-valuenow={Math.round(shown())}
      aria-valuetext={`${formatTime(shown())} de ${formatTime(props.duration)}`}
      class="group relative flex h-5 cursor-pointer touch-none items-center"
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={() => setDrag(null)}
      onPointerLeave={() => setHover(null)}
      onKeyDown={key}
    >
      <div class="relative h-1 w-full rounded-full bg-white/18 transition-[height] duration-150 group-hover:h-1.5">
        <div class="absolute inset-y-0 left-0 rounded-full bg-white/32" style={{ width: pct(props.buffered) }} />
        <div class="absolute inset-y-0 left-0 rounded-full bg-accent" style={{ width: pct(shown()) }} />
        <For each={props.chapters.filter((c) => c > 0)}>
          {(c) => <div class="absolute -inset-y-px w-0.5 bg-black/80" style={{ left: pct(c) }} />}
        </For>
      </div>
      <div
        class="pointer-events-none absolute size-3.5 -translate-x-1/2 rounded-full bg-white shadow-md transition-transform duration-150 group-hover:scale-110"
        classList={{ "scale-0": !props.duration }}
        style={{ left: pct(shown()) }}
      />
      <Show when={hover()}>
        {(h) => (
          <div
            class="pointer-events-none absolute bottom-6 -translate-x-1/2 rounded-md bg-black/85 px-2 py-0.5 text-xs font-medium tabular-nums"
            style={{ left: `${h().x}px` }}
          >
            {formatTime(h().t)}
          </div>
        )}
      </Show>
    </div>
  );
}
