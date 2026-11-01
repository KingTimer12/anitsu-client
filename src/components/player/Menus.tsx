import { Check } from "lucide-solid";
import { For, type JSX, Show } from "solid-js";
import type { Progress, Settings } from "../../lib/player/api";
import { episodeNumber, type Track, videoFiles } from "../../lib/player/logic";
import type { FileEntry } from "../../lib/anitsu";
import { parseEpisode } from "../../lib/library";
import { cx } from "../ui";

/** Painel flutuante acima dos controles, à direita. */
function Panel(props: { label: string; class?: string; children: JSX.Element }) {
  return (
    <div
      role="dialog"
      aria-label={props.label}
      onClick={(e) => e.stopPropagation()}
      class={cx(
        "scrollbar-thin absolute right-4 bottom-28 z-20 max-h-[min(70vh,520px)] animate-pop overflow-y-auto rounded-2xl border border-line-strong bg-surface/92 p-2 shadow-2xl backdrop-blur-xl",
        props.class,
      )}
    >
      {props.children}
    </div>
  );
}

function Heading(props: { children: JSX.Element }) {
  return <p class="px-3 pt-2 pb-1 text-xs font-medium text-faint">{props.children}</p>;
}

function Option(props: { selected: boolean; onClick(): void; children: JSX.Element; hint?: string }) {
  return (
    <button
      type="button"
      role="menuitemradio"
      aria-checked={props.selected}
      onClick={props.onClick}
      class={cx(
        "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-white/8",
        props.selected && "text-accent-ink",
      )}
    >
      <Check class={cx("size-4 shrink-0", !props.selected && "invisible")} aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate">{props.children}</span>
      <Show when={props.hint}>
        <span class="text-xs text-faint uppercase">{props.hint}</span>
      </Show>
    </button>
  );
}

const trackName = (t: Track) => t.title || t.lang?.toUpperCase() || `Faixa ${t.id}`;

/** Legendas (com "Desligada") e áudio do arquivo. */
export function TracksMenu(props: {
  tracks: Track[];
  onSub(t: Track | null): void;
  onAudio(t: Track): void;
}) {
  const subs = () => props.tracks.filter((t) => t.type === "sub");
  const audios = () => props.tracks.filter((t) => t.type === "audio");
  return (
    <Panel label="Legendas e áudio" class="w-72">
      <div role="menu" aria-label="Legendas">
        <Heading>Legendas</Heading>
        <Option selected={!subs().some((t) => t.selected)} onClick={() => props.onSub(null)}>
          Desligada
        </Option>
        <For each={subs()}>
          {(t) => (
            <Option selected={t.selected} onClick={() => props.onSub(t)} hint={t.title ? t.lang : undefined}>
              {trackName(t)}
            </Option>
          )}
        </For>
      </div>
      <Show when={audios().length > 1}>
        <div role="menu" aria-label="Áudio" class="mt-1 border-t border-line pt-1">
          <Heading>Áudio</Heading>
          <For each={audios()}>
            {(t) => (
              <Option selected={t.selected} onClick={() => props.onAudio(t)} hint={t.title ? t.lang : undefined}>
                {trackName(t)}
              </Option>
            )}
          </For>
        </div>
      </Show>
    </Panel>
  );
}

function Toggle(props: { label: string; checked: boolean; onChange(v: boolean): void }) {
  return (
    <label class="flex cursor-pointer items-center justify-between gap-4 rounded-lg px-3 py-2.5 text-sm hover:bg-white/6">
      {props.label}
      <input
        type="checkbox"
        role="switch"
        class="peer sr-only"
        checked={props.checked}
        onChange={(e) => props.onChange(e.currentTarget.checked)}
      />
      <span
        aria-hidden="true"
        class="relative h-5 w-9 shrink-0 rounded-full bg-white/15 transition-colors peer-checked:bg-accent peer-focus-visible:outline-2 peer-focus-visible:outline-accent-ink after:absolute after:top-0.5 after:left-0.5 after:size-4 after:rounded-full after:bg-white after:transition-transform peer-checked:after:translate-x-4"
      />
    </label>
  );
}

function Choice<T extends string | number>(props: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange(v: T): void;
}) {
  return (
    <div class="px-3 py-2.5 text-sm">
      <p class="mb-2">{props.label}</p>
      <div class="flex gap-1.5" role="radiogroup" aria-label={props.label}>
        <For each={props.options}>
          {(o) => (
            <button
              type="button"
              role="radio"
              aria-checked={o.value === props.value}
              onClick={() => props.onChange(o.value)}
              class={cx(
                "flex-1 rounded-lg border px-2 py-1.5 text-xs font-medium transition",
                o.value === props.value
                  ? "border-accent bg-accent-soft text-accent-ink"
                  : "border-line-strong text-muted hover:text-fg",
              )}
            >
              {o.label}
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

const LANGS = [
  { value: "por", label: "Português" },
  { value: "eng", label: "Inglês" },
  { value: "jpn", label: "Japonês" },
  { value: "kor", label: "Coreano" },
  { value: "spa", label: "Espanhol" },
];

function LangSelect(props: { label: string; value: string; onChange(v: string): void }) {
  return (
    <label class="flex items-center justify-between gap-4 px-3 py-2.5 text-sm">
      {props.label}
      <select
        value={props.value}
        onChange={(e) => props.onChange(e.currentTarget.value)}
        class="rounded-lg border border-line-strong bg-elevated px-2 py-1 text-sm text-fg"
      >
        <For each={LANGS}>{(l) => <option value={l.value}>{l.label}</option>}</For>
      </select>
    </label>
  );
}

/** Configurações do player (salvas no `player.json`). */
export function SettingsMenu(props: { settings: Settings; onChange(s: Settings): void }) {
  const set = <K extends keyof Settings>(k: K, v: Settings[K]) => props.onChange({ ...props.settings, [k]: v });
  return (
    <Panel label="Configurações do player" class="w-80">
      <Toggle label="Ir para o próximo episódio sozinho" checked={props.settings.autoNext} onChange={(v) => set("autoNext", v)} />
      <Show when={props.settings.autoNext}>
        <Choice
          label="Tempo da contagem"
          value={props.settings.countdownSec}
          options={[5, 10, 15].map((n) => ({ value: n, label: `${n} s` }))}
          onChange={(v) => set("countdownSec", v)}
        />
      </Show>
      <Toggle
        label='Mostrar "Pular abertura"'
        checked={props.settings.showSkipIntro}
        onChange={(v) => set("showSkipIntro", v)}
      />
      <div class="my-1 border-t border-line" />
      <LangSelect label="Legenda preferida" value={props.settings.subLang} onChange={(v) => set("subLang", v)} />
      <LangSelect label="Áudio preferido" value={props.settings.audioLang} onChange={(v) => set("audioLang", v)} />
      <div class="my-1 border-t border-line" />
      <Choice
        label="Memória à frente"
        value={props.settings.cacheMib}
        options={[100, 150, 300, 500].map((n) => ({ value: n, label: `${n} MB` }))}
        onChange={(v) => set("cacheMib", v)}
      />
    </Panel>
  );
}

/** Lista de episódios da pasta, com progresso salvo. */
export function EpisodesPanel(props: {
  files: FileEntry[];
  current: string;
  progressOf(name: string): Progress | undefined;
  onPick(f: FileEntry): void;
}) {
  return (
    <Panel label="Episódios" class="w-96">
      <Heading>Episódios</Heading>
      <ul>
        <For each={videoFiles(props.files)}>
          {(f) => {
            const n = episodeNumber(f.name);
            const p = () => props.progressOf(f.name);
            const isCurrent = () => f.name === props.current;
            return (
              <li>
                <button
                  type="button"
                  onClick={() => props.onPick(f)}
                  aria-current={isCurrent() ? "true" : undefined}
                  title={f.name}
                  class={cx(
                    "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors hover:bg-white/8",
                    isCurrent() && "bg-accent-soft",
                  )}
                >
                  <span
                    class={cx(
                      "w-8 shrink-0 text-center font-display text-lg font-semibold tabular-nums",
                      isCurrent() ? "text-accent-ink" : "text-muted",
                    )}
                  >
                    {n ?? "·"}
                  </span>
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-sm">{n !== null ? `Episódio ${n}` : parseEpisode(f.name).label}</span>
                    <Show when={p()}>
                      {(pr) => (
                        <span class="mt-1 block h-0.5 overflow-hidden rounded-full bg-white/12">
                          <span
                            class="block h-full bg-accent"
                            style={{ width: `${pr().watched ? 100 : Math.min(100, (pr().position / pr().duration) * 100)}%` }}
                          />
                        </span>
                      )}
                    </Show>
                  </span>
                  <Show when={p()?.watched}>
                    <Check class="size-4 text-ok" aria-label="Visto" />
                  </Show>
                </button>
              </li>
            );
          }}
        </For>
      </ul>
    </Panel>
  );
}
