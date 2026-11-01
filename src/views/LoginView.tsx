import { ExternalLink, LoaderCircle } from "lucide-solid";
import { createResource, createSignal, For, Show } from "solid-js";
import { TitleBar } from "../components/TitleBar";
import { Button } from "../components/ui";
import { anilist } from "../lib/anilist/api";
import { AnitsuError, auth } from "../lib/anitsu";

/** Logo do Discord (marca oficial, simplificada em SVG). */
function DiscordMark(props: { class?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" class={props.class}>
      <path d="M20.32 4.37A19.8 19.8 0 0 0 15.39 2.8a.07.07 0 0 0-.08.04c-.21.38-.45.87-.61 1.25a18.3 18.3 0 0 0-5.49 0 12.6 12.6 0 0 0-.62-1.25.08.08 0 0 0-.08-.04 19.7 19.7 0 0 0-4.93 1.53.07.07 0 0 0-.03.03C.53 9.05-.32 13.58.1 18.06a.08.08 0 0 0 .03.06 19.9 19.9 0 0 0 5.99 3.03.08.08 0 0 0 .08-.03c.46-.63.87-1.3 1.23-2a.08.08 0 0 0-.04-.1 13.1 13.1 0 0 1-1.87-.9.08.08 0 0 1-.01-.13l.37-.29a.07.07 0 0 1 .08-.01c3.93 1.8 8.18 1.8 12.07 0a.07.07 0 0 1 .08.01l.37.29a.08.08 0 0 1-.01.13c-.6.35-1.22.65-1.87.9a.08.08 0 0 0-.04.1c.36.7.77 1.37 1.23 2a.08.08 0 0 0 .08.03 19.8 19.8 0 0 0 6-3.03.08.08 0 0 0 .03-.06c.5-5.18-.84-9.67-3.55-13.66a.06.06 0 0 0-.03-.03ZM8.02 15.33c-1.18 0-2.16-1.09-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.33-.96 2.42-2.16 2.42Zm7.97 0c-1.18 0-2.15-1.09-2.15-2.42 0-1.33.95-2.42 2.15-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.33-.95 2.42-2.16 2.42Z" />
    </svg>
  );
}

export function LoginView() {
  const [season] = createResource(() => anilist.season().catch(() => []));
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal("");

  async function login() {
    setBusy(true);
    setError("");
    try {
      await auth.login();
    } catch (e) {
      const err = AnitsuError.from(e);
      if (err.kind !== "cancelled") setError(err.message);
    } finally {
      setBusy(false);
    }
  }

  const covers = () => (season() ?? []).filter((m) => m.coverImage?.large).slice(0, 30);

  return (
    <div class="flex h-full flex-col bg-bg">
      <TitleBar />
      <div class="relative flex-1 overflow-hidden">
        {/* Parede de capas da temporada, inclinada. */}
        <div aria-hidden="true" class="absolute -inset-y-1/4 right-[-10%] left-[38%] grid -rotate-[8deg] grid-cols-5 gap-4 opacity-60">
          <For each={covers()}>
            {(m, i) => (
              <img
                src={m.coverImage!.large!}
                alt=""
                class="aspect-[2/3] w-full animate-fade-in rounded-2xl object-cover even:translate-y-1/3"
                style={{ "background-color": m.coverImage?.color ?? "#18181f", "animation-delay": `${i() * 30}ms` }}
              />
            )}
          </For>
        </div>
        <div
          aria-hidden="true"
          class="absolute inset-0 bg-[linear-gradient(90deg,var(--color-bg)_32%,rgb(7_7_10/0.85)_52%,rgb(7_7_10/0.25)_100%),linear-gradient(0deg,var(--color-bg),transparent_35%)]"
        />

        <main class="relative z-10 flex h-full items-center px-[clamp(32px,8vw,120px)]">
          <div class="w-full max-w-md animate-fade-up">
            <h1 class="font-display text-7xl font-bold tracking-tighter text-brand">higanbana</h1>
            <p class="mt-4 max-w-[36ch] text-lg text-muted">
              A nuvem de animes da Anitsu, com capa, sinopse e ficha técnica de cada título.
            </p>

            <div class="mt-10">
              <Show
                when={busy()}
                fallback={
                  <button
                    type="button"
                    onClick={login}
                    class="inline-flex h-12 items-center gap-3 rounded-2xl bg-[#5865F2] px-6 text-base font-semibold text-white shadow-[0_10px_40px_-10px_#5865F2] transition duration-200 ease-expo hover:brightness-110 active:scale-[0.97]"
                  >
                    <DiscordMark class="size-5" />
                    Entrar com Discord
                  </button>
                }
              >
                <div role="status" class="rounded-2xl border border-line-strong bg-surface/80 p-5 backdrop-blur-xl">
                  <div class="flex items-center gap-3">
                    <LoaderCircle class="size-5 animate-spin text-brand" aria-hidden="true" />
                    <p class="font-semibold">Continue no seu navegador</p>
                  </div>
                  <p class="mt-2 text-sm text-muted">
                    Abrimos o Discord no navegador em que você já está logado. Clique em Autorizar e volte para cá.
                  </p>
                  <div class="mt-4 flex gap-2">
                    <Button onClick={() => auth.reopenLogin()} class="h-9">
                      <ExternalLink class="size-4" aria-hidden="true" />
                      Abrir de novo
                    </Button>
                    <Button variant="ghost" onClick={() => auth.cancelLogin()} class="h-9">
                      Cancelar
                    </Button>
                  </div>
                </div>
              </Show>
              <Show when={error()}>
                <p role="alert" class="mt-4 text-sm text-danger">
                  {error()} Tente entrar de novo.
                </p>
              </Show>
            </div>

            <p class="mt-16 text-xs text-faint">Capas e informações do AniList.</p>
          </div>
        </main>
      </div>
    </div>
  );
}
