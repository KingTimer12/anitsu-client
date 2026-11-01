import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChevronLeft, ChevronRight, Copy, Minus, Square, X } from "lucide-solid";
import { createSignal, onCleanup, onMount, Show, type JSX } from "solid-js";
import { router } from "../lib/router";
import { cx, IconButton } from "./ui";

const appWindow = getCurrentWindow();

/** Botões de janela no padrão do Windows 11 (fechar fica vermelho no hover). */
function WindowControls() {
  const [maximized, setMaximized] = createSignal(false);

  onMount(() => {
    // onCleanup precisa ser registrado de forma síncrona (dentro do dono do componente).
    let unlisten: (() => void) | undefined;
    let disposed = false;
    onCleanup(() => {
      disposed = true;
      unlisten?.();
    });
    void appWindow.isMaximized().then(setMaximized);
    void appWindow
      .onResized(async () => setMaximized(await appWindow.isMaximized()))
      .then((fn) => (disposed ? fn() : (unlisten = fn)));
  });

  const base = "inline-flex h-full w-12 items-center justify-center text-muted transition-colors duration-100 hover:text-fg";
  return (
    <div class="flex h-full self-stretch">
      <button type="button" class={cx(base, "hover:bg-white/8")} aria-label="Minimizar" onClick={() => appWindow.minimize()}>
        <Minus class="size-4" stroke-width={1.5} />
      </button>
      <button
        type="button"
        class={cx(base, "hover:bg-white/8")}
        aria-label={maximized() ? "Restaurar" : "Maximizar"}
        onClick={() => appWindow.toggleMaximize()}
      >
        <Show when={maximized()} fallback={<Square class="size-3.5" stroke-width={1.5} />}>
          <Copy class="size-3.5 -scale-x-100" stroke-width={1.5} />
        </Show>
      </button>
      <button
        type="button"
        class={cx(base, "hover:bg-close hover:text-white")}
        aria-label="Fechar"
        onClick={() => appWindow.close()}
      >
        <X class="size-4" stroke-width={1.5} />
      </button>
    </div>
  );
}

/**
 * Barra de título própria (janela sem decoração do sistema).
 * Áreas com `data-tauri-drag-region` arrastam a janela; duplo clique maximiza.
 */
export function TitleBar(props: { center?: JSX.Element; actions?: JSX.Element; showNav?: boolean }) {
  return (
    <header
      data-tauri-drag-region
      class="relative z-40 flex h-12 shrink-0 items-center gap-1 border-b border-line bg-bg/85 pl-3 backdrop-blur-xl"
    >
      <button
        type="button"
        onClick={() => router.go("")}
        class="mr-2 rounded-md px-1 font-display text-xl font-bold tracking-tight text-brand"
        aria-label="Anitsu, ir para o início"
      >
        anitsu
      </button>

      <Show when={props.showNav}>
        <IconButton label="Voltar (Alt+←)" onClick={router.back} disabled={!router.canBack()} class="size-8">
          <ChevronLeft class="size-4.5" />
        </IconButton>
        <IconButton label="Avançar (Alt+→)" onClick={router.forward} disabled={!router.canForward()} class="size-8">
          <ChevronRight class="size-4.5" />
        </IconButton>
      </Show>

      <div data-tauri-drag-region class="flex h-full min-w-0 flex-1 items-center justify-center px-4">
        {props.center}
      </div>

      <div class="flex items-center gap-1 pr-2">{props.actions}</div>
      <WindowControls />
    </header>
  );
}
