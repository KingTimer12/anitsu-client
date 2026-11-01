import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, LoaderCircle, WifiOff } from "lucide-solid";
import { batch, createEffect, createSignal, getOwner, on, onCleanup, onMount, runWithOwner, Show } from "solid-js";
import { Controls } from "../components/player/Controls";
import { EpisodesPanel, SettingsMenu, TracksMenu } from "../components/player/Menus";
import { NextCard, SkipButton } from "../components/player/Overlays";
import { SeekBar } from "../components/player/SeekBar";
import { WindowControls } from "../components/TitleBar";
import {
  type Chapter,
  classifyChapters,
  episodeNumber,
  isWatched,
  nextCardStart,
  nextEpisode,
  pickTracks,
  type SkipTimes,
  type Track,
} from "../lib/player/logic";
import { createMpv, type Mpv } from "../lib/player/mpv";
import { player, type PlayTarget } from "../lib/player/session";
import { playerStore } from "../lib/player/store";
import { playerApi, type Settings } from "../lib/player/api";
import { joinPath } from "../lib/anitsu";
import { cx } from "../components/ui";

const HIDE_AFTER_MS = 2500;
const appWindow = getCurrentWindow();

type Menu = "tracks" | "settings" | "episodes" | null;

/** Player "Imersivo": o mpv desenha atrás do webview; esta tela é só a interface por cima. */
export function PlayerView(props: { target: PlayTarget; onClose: () => void }) {
  const [pos, setPos] = createSignal(0);
  const [dur, setDur] = createSignal(0);
  const [paused, setPaused] = createSignal(false);
  const [volume, setVolume] = createSignal(100);
  const [muted, setMuted] = createSignal(false);
  const [buffering, setBuffering] = createSignal(true);
  const [cacheAhead, setCacheAhead] = createSignal(0);
  const [chapters, setChapters] = createSignal<Chapter[]>([]);
  const [tracks, setTracks] = createSignal<Track[]>([]);
  const [eof, setEof] = createSignal(false);
  const [fullscreen, setFullscreen] = createSignal(false);
  const [menu, setMenu] = createSignal<Menu>(null);
  const [idle, setIdle] = createSignal(false);
  const [netError, setNetError] = createSignal(false);
  const [skip, setSkip] = createSignal<SkipTimes>({ op: null, ed: null });
  const [nextCancelled, setNextCancelled] = createSignal(false);
  let skipResolvedFor = "";
  let tracksAppliedFor = "";

  let mpv: Mpv | undefined;
  /** Caminho do arquivo tocando; guardado fora das props para o cleanup (props já desmontadas). */
  let playingPath = "";
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let retryDelay = 2000;

  const file = () => props.target.file;
  const remotePath = () => joinPath(props.target.dir, file().name);
  const next = () => nextEpisode(props.target.files, file().name);
  const epLabel = () => {
    const n = episodeNumber(file().name);
    return n !== null ? `Ep. ${n}` : file().name;
  };

  // --- Progresso ---
  function recordProgress() {
    const d = dur();
    if (!d || !playingPath) return;
    playerStore.setProgress(playingPath, {
      position: pos(),
      duration: d,
      watched: isWatched(pos(), d, skip().ed),
      updatedAt: Math.floor(Date.now() / 1000),
    });
  }

  // --- mpv ---
  function onProp(name: string, value: unknown) {
    switch (name) {
      case "time-pos":
        if (typeof value === "number") {
          setPos(value);
          recordProgress();
        }
        break;
      case "duration":
        setDur(typeof value === "number" ? value : 0);
        break;
      case "pause":
        setPaused(!!value);
        if (value) void playerStore.flush();
        break;
      case "volume":
        setVolume(Math.round(Number(value)));
        break;
      case "mute":
        setMuted(!!value);
        break;
      case "paused-for-cache":
        setBuffering(!!value);
        break;
      case "demuxer-cache-time":
        setCacheAhead(typeof value === "number" ? value : 0);
        break;
      case "chapter-list":
        setChapters(Array.isArray(value) ? (value as Chapter[]) : []);
        break;
      case "track-list":
        setTracks(Array.isArray(value) ? (value as Track[]) : []);
        break;
      case "eof-reached":
        setEof(!!value);
        break;
    }
  }

  function onEnd(reason: string) {
    if (reason !== "error") return;
    // Rede caiu: tenta de novo na mesma posição, esperando cada vez mais.
    setNetError(true);
    clearTimeout(retryTimer);
    retryTimer = setTimeout(() => void loadCurrent(pos()), retryDelay);
    retryDelay = Math.min(retryDelay * 2, 30_000);
  }

  async function loadCurrent(startAt?: number) {
    if (!mpv) return;
    playingPath = remotePath();
    const saved = playerStore.progress(playingPath);
    const start = startAt ?? (saved && !saved.watched ? saved.position : 0);
    batch(() => {
      setBuffering(true);
      setEof(false);
      setChapters([]);
      setTracks([]);
      if (startAt === undefined) {
        setSkip({ op: null, ed: null });
        setNextCancelled(false);
        skipResolvedFor = "";
        tracksAppliedFor = "";
      }
      if (startAt === undefined) {
        setPos(start);
        setDur(0);
      }
    });
    try {
      const url = await playerApi.streamUrl(remotePath(), file().size);
      await mpv.load(url, start);
      setNetError(false);
      retryDelay = 2000;
    } catch {
      onEnd("error");
    }
  }

  const owner = getOwner();
  let disposed = false;
  onCleanup(() => (disposed = true));
  onMount(async () => {
    try {
      mpv = await createMpv({ "demuxer-max-bytes": `${playerStore.settings().cacheMib}MiB` }, { onProp, onEnd });
    } catch (e) {
      console.error("mpv", e);
      setNetError(true);
      return;
    }
    if (disposed) {
      void mpv.destroy().catch(() => {});
      mpv = undefined;
      return;
    }
    // Carrega o episódio inicial e cada troca (próximo, lista de episódios).
    // Depois do `await` não há dono reativo: reata ao do componente para o efeito morrer junto.
    runWithOwner(owner, () => createEffect(on(() => file().name, () => void loadCurrent())));
    void appWindow.isFullscreen().then(setFullscreen);
  });

  onCleanup(() => {
    // Primeiro parar o vídeo; nada aqui pode ler props (o <Show> já desmontou).
    void mpv?.destroy().catch(() => {});
    mpv = undefined;
    clearTimeout(retryTimer);
    recordProgress();
    void playerStore.flush();
    if (fullscreen()) void appWindow.setFullscreen(false);
  });

  // --- Ações ---
  const togglePause = () => void mpv?.set("pause", !paused());
  const seek = (t: number) => void mpv?.command("seek", Math.max(0, Math.min(t, dur() - 0.5)), "absolute+exact");
  const setVol = (v: number) => {
    const clamped = Math.max(0, Math.min(100, v));
    void mpv?.set("volume", clamped);
    if (muted() && clamped > 0) void mpv?.set("mute", false);
  };
  const toggleMute = () => void mpv?.set("mute", !muted());
  async function toggleFullscreen() {
    const fs = !fullscreen();
    await appWindow.setFullscreen(fs);
    setFullscreen(fs);
  }
  function goNext() {
    const n = next();
    if (!n) return;
    recordProgress();
    void playerStore.flush();
    player.switchTo(n);
  }
  const toggleMenu = (m: Exclude<Menu, null>) => setMenu((cur) => (cur === m ? null : m));

  // --- Abertura/encerramento: capítulos do arquivo, senão AniSkip (só animes) ---
  createEffect(() => {
    const d = dur();
    const name = file().name;
    if (!d || skipResolvedFor === name) return;
    skipResolvedFor = name;
    const fromChapters = classifyChapters(chapters(), d);
    if (fromChapters.op || fromChapters.ed) return setSkip(fromChapters);
    const ep = episodeNumber(name);
    const { malId, isAnime } = props.target;
    if (!isAnime || !malId || ep === null) return;
    void playerApi
      .aniskip(malId, ep, d)
      .then((t) => file().name === name && setSkip(t))
      .catch(() => {});
  });

  const settings = () => playerStore.settings();
  const showSkip = () => {
    const op = skip().op;
    return settings().showSkipIntro && !!op && pos() >= op.start && pos() < op.end - 1;
  };
  const skipIntro = () => {
    const op = skip().op;
    if (op && showSkip()) seek(op.end);
  };

  // --- Próximo episódio ---
  const showNext = () =>
    !!next() && dur() > 0 && !nextCancelled() && pos() >= nextCardStart(dur(), skip().ed, settings().countdownSec);
  createEffect(() => {
    if (eof() && next() && settings().autoNext && !nextCancelled()) goNext();
  });
  const nextLabel = () => {
    const n = next();
    if (!n) return "";
    const num = episodeNumber(n.name);
    return num !== null ? `Episódio ${num}` : n.name;
  };

  // --- Faixas: escolha lembrada do release ou idiomas preferidos ---
  createEffect(() => {
    const list = tracks();
    const name = file().name;
    // Só com o arquivo carregado (duração conhecida): antes disso o mpv recusa sid/aid.
    if (!mpv || !dur() || !list.some((t) => t.type !== "video") || tracksAppliedFor === name) return;
    tracksAppliedFor = name;
    const { sid, aid } = pickTracks(list, playerStore.tracks(props.target.dir), settings());
    if (sid !== null) setTrack("sid", sid);
    if (aid !== null) setTrack("aid", aid);
  });
  /** `sid`/`aid` precisam de inteiro; o plugin manda números como double, então vai como texto. */
  const setTrack = (prop: "sid" | "aid", id: number | "no") =>
    void mpv?.command("set", prop, String(id)).catch(() => {});
  function chooseSub(t: Track | null) {
    setTrack("sid", t ? t.id : "no");
    const prev = playerStore.tracks(props.target.dir);
    playerStore.setTracks(props.target.dir, {
      ...prev,
      subOff: !t,
      subLang: t?.lang,
      subTitle: t?.title,
    });
  }
  function chooseAudio(t: Track) {
    setTrack("aid", t.id);
    const prev = playerStore.tracks(props.target.dir);
    playerStore.setTracks(props.target.dir, { subOff: false, ...prev, audioLang: t.lang, audioTitle: t.title });
  }
  function changeSettings(s: Settings) {
    if (s.cacheMib !== settings().cacheMib) void mpv?.set("demuxer-max-bytes", `${s.cacheMib}MiB`);
    playerStore.setSettings(s);
  }

  // --- Esconder controles com o mouse parado ---
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  function poke() {
    setIdle(false);
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => setIdle(true), HIDE_AFTER_MS);
  }
  onCleanup(() => clearTimeout(hideTimer));
  const controlsHidden = () => idle() && !paused() && !menu() && !buffering();

  // --- Atalhos ---
  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t?.closest("input, select, textarea")) return;
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    let handled = true;
    switch (e.key) {
      case " ":
        togglePause();
        break;
      case "ArrowLeft":
        seek(pos() - 5);
        break;
      case "ArrowRight":
        seek(pos() + 5);
        break;
      case "ArrowUp":
        setVol(volume() + 5);
        break;
      case "ArrowDown":
        setVol(volume() - 5);
        break;
      case "m":
      case "M":
        toggleMute();
        break;
      case "f":
      case "F":
        void toggleFullscreen();
        break;
      case "n":
      case "N":
        goNext();
        break;
      case "s":
      case "S":
        skipIntro();
        break;
      case "Escape":
        if (menu()) setMenu(null);
        else if (fullscreen()) void toggleFullscreen();
        else props.onClose();
        break;
      default:
        handled = false;
    }
    if (handled) {
      e.preventDefault();
      poke();
    }
  }
  onMount(() => {
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
    poke();
  });

  // Ao chegar ao fim sem próximo episódio, mostra os controles.
  createEffect(() => eof() && setIdle(false));

  return (
    <div
      class={cx("fixed inset-0 z-50 flex flex-col select-none", controlsHidden() && "cursor-none")}
      onMouseMove={poke}
      onPointerDown={poke}
    >
      {/* Barra de título sobreposta */}
      <header
        data-tauri-drag-region
        class={cx(
          "relative z-10 flex h-12 shrink-0 items-center gap-2 bg-gradient-to-b from-black/75 to-transparent pl-2 transition-opacity duration-300",
          controlsHidden() && "pointer-events-none opacity-0",
        )}
      >
        <button
          type="button"
          onClick={props.onClose}
          aria-label="Voltar"
          title="Voltar (Esc)"
          class="grid size-9 place-items-center rounded-lg text-fg transition hover:bg-white/10 active:scale-95"
        >
          <ArrowLeft class="size-5" />
        </button>
        <p data-tauri-drag-region class="min-w-0 flex-1 truncate text-sm">
          <span class="font-display font-semibold">{props.target.animeTitle}</span>
          <span class="text-muted"> · {epLabel()}</span>
        </p>
        <Show when={!fullscreen()}>
          <WindowControls />
        </Show>
      </header>

      {/* Área do vídeo: clique pausa, duplo clique alterna tela cheia */}
      <div
        class="relative min-h-0 flex-1"
        onClick={() => (menu() ? setMenu(null) : togglePause())}
        onDblClick={() => void toggleFullscreen()}
      >
        <Show when={buffering() && !netError()}>
          <div class="pointer-events-none absolute inset-0 grid place-items-center" role="status" aria-label="Carregando">
            <LoaderCircle class="size-10 animate-spin text-white/80 drop-shadow-lg" />
          </div>
        </Show>
        <Show when={netError()}>
          <div class="absolute inset-0 grid place-items-center" role="alert">
            <div class="flex items-center gap-3 rounded-xl border border-line-strong bg-black/75 px-5 py-3 text-sm backdrop-blur">
              <WifiOff class="size-5 text-danger" aria-hidden="true" />
              Sem conexão com a nuvem. Tentando de novo…
            </div>
          </div>
        </Show>
      </div>

      {/* Pular abertura / próximo episódio: acima dos controles, mesmo com eles escondidos */}
      <div
        class={cx(
          "pointer-events-none absolute right-5 z-20 flex flex-col items-end gap-3 transition-[bottom] duration-300 ease-expo *:pointer-events-auto",
          controlsHidden() ? "bottom-6" : "bottom-28",
        )}
      >
        <Show when={showSkip() && !showNext()}>
          <SkipButton onSkip={skipIntro} />
        </Show>
        <Show when={showNext()}>
          <NextCard
            nextLabel={nextLabel()}
            countdown={settings().autoNext ? settings().countdownSec : null}
            onPlay={goNext}
            onCancel={() => setNextCancelled(true)}
          />
        </Show>
      </div>

      <Show when={menu() === "tracks"}>
        <TracksMenu tracks={tracks()} onSub={chooseSub} onAudio={chooseAudio} />
      </Show>
      <Show when={menu() === "settings"}>
        <SettingsMenu settings={settings()} onChange={changeSettings} />
      </Show>
      <Show when={menu() === "episodes"}>
        <EpisodesPanel
          files={props.target.files}
          current={file().name}
          progressOf={(name) => playerStore.progress(joinPath(props.target.dir, name))}
          onPick={(f) => {
            setMenu(null);
            recordProgress();
            void playerStore.flush();
            player.switchTo(f);
          }}
        />
      </Show>

      {/* Controles inferiores */}
      <footer
        class={cx(
          "relative z-10 bg-gradient-to-t from-black/85 via-black/55 to-transparent px-5 pt-10 pb-3 transition-opacity duration-300",
          controlsHidden() && "pointer-events-none opacity-0",
        )}
      >
        <SeekBar
          position={pos()}
          duration={dur()}
          buffered={pos() + cacheAhead()}
          chapters={chapters().map((c) => c.time)}
          onSeek={seek}
        />
        <Controls
          paused={paused()}
          volume={volume()}
          muted={muted()}
          position={pos()}
          duration={dur()}
          title={epLabel()}
          hasNext={!!next()}
          fullscreen={fullscreen()}
          menu={menu()}
          on={{
            togglePause,
            next: goNext,
            setVolume: setVol,
            toggleMute,
            openTracks: () => toggleMenu("tracks"),
            openSettings: () => toggleMenu("settings"),
            openEpisodes: () => toggleMenu("episodes"),
            toggleFullscreen: () => void toggleFullscreen(),
          }}
        />
      </footer>
    </div>
  );
}
