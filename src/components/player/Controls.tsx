import {
  Captions,
  ListVideo,
  Maximize,
  Minimize,
  Pause,
  Play,
  Settings2,
  SkipForward,
  Volume1,
  Volume2,
  VolumeX,
} from "lucide-solid";
import { Show, type JSX } from "solid-js";
import { formatTime } from "../../lib/player/logic";
import { cx } from "../ui";

export interface ControlsHandlers {
  togglePause(): void;
  next(): void;
  setVolume(v: number): void;
  toggleMute(): void;
  openTracks(): void;
  openSettings(): void;
  openEpisodes(): void;
  toggleFullscreen(): void;
}

function Btn(props: { label: string; onClick: () => void; disabled?: boolean; active?: boolean; children: JSX.Element }) {
  return (
    <button
      type="button"
      aria-label={props.label}
      title={props.label}
      disabled={props.disabled}
      onClick={props.onClick}
      class={cx(
        "grid size-10 shrink-0 place-items-center rounded-lg text-fg transition duration-150 hover:bg-white/10 active:scale-95 disabled:pointer-events-none disabled:opacity-35",
        props.active && "text-accent-ink",
      )}
    >
      {props.children}
    </button>
  );
}

/** Linha de controles do layout "Imersivo". */
export function Controls(props: {
  paused: boolean;
  volume: number;
  muted: boolean;
  position: number;
  duration: number;
  title: string;
  hasNext: boolean;
  fullscreen: boolean;
  menu: "tracks" | "settings" | "episodes" | null;
  on: ControlsHandlers;
}) {
  const VolIcon = () => (props.muted || props.volume === 0 ? VolumeX : props.volume < 50 ? Volume1 : Volume2);
  return (
    <div class="flex items-center gap-1">
      <Btn label={props.paused ? "Reproduzir (Espaço)" : "Pausar (Espaço)"} onClick={props.on.togglePause}>
        <Show when={props.paused} fallback={<Pause class="size-5.5" fill="currentColor" stroke-width={0} />}>
          <Play class="size-5.5" fill="currentColor" stroke-width={0} />
        </Show>
      </Btn>
      <Btn label="Próximo episódio (N)" onClick={props.on.next} disabled={!props.hasNext}>
        <SkipForward class="size-5" />
      </Btn>

      <div class="group/vol flex items-center">
        <Btn label={props.muted ? "Ativar som (M)" : "Silenciar (M)"} onClick={props.on.toggleMute}>
          {(() => {
            const I = VolIcon();
            return <I class="size-5" />;
          })()}
        </Btn>
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          aria-label="Volume"
          value={props.muted ? 0 : props.volume}
          onInput={(e) => props.on.setVolume(Number(e.currentTarget.value))}
          class="volume-range w-0 opacity-0 transition-all duration-200 group-hover/vol:w-24 group-hover/vol:opacity-100 focus-visible:w-24 focus-visible:opacity-100"
          style={{ "--vol": `${props.muted ? 0 : props.volume}%` }}
        />
      </div>

      <span class="ml-2 text-sm text-muted tabular-nums">
        {formatTime(props.position)} <span class="text-faint">/</span> {formatTime(props.duration)}
      </span>

      <span class="min-w-0 flex-1 truncate px-4 text-center font-display text-sm font-medium">{props.title}</span>

      <Btn label="Legendas e áudio" onClick={props.on.openTracks} active={props.menu === "tracks"}>
        <Captions class="size-5" />
      </Btn>
      <Btn label="Configurações do player" onClick={props.on.openSettings} active={props.menu === "settings"}>
        <Settings2 class="size-5" />
      </Btn>
      <Btn label="Episódios" onClick={props.on.openEpisodes} active={props.menu === "episodes"}>
        <ListVideo class="size-5" />
      </Btn>
      <Btn label={props.fullscreen ? "Sair da tela cheia (F)" : "Tela cheia (F)"} onClick={props.on.toggleFullscreen}>
        <Show when={props.fullscreen} fallback={<Maximize class="size-5" />}>
          <Minimize class="size-5" />
        </Show>
      </Btn>
    </div>
  );
}
