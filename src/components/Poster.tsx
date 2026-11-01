import { FolderCheck, Star } from "lucide-solid";
import { createResource, createSignal, onCleanup, onMount, Show, type JSX } from "solid-js";
import { lookupMedia } from "../lib/anilist/api";
import { label, mediaTitle, mediaYear } from "../lib/anilist/text";
import type { MediaSummary, MediaType } from "../lib/anilist/types";
import { folderTitle, folderYear } from "../lib/library";
import { cx } from "./ui";

/** Capa tipográfica para títulos sem correspondência no AniList. */
function TypePoster(props: { name: string }) {
  return (
    <div class="flex size-full items-end bg-[radial-gradient(120%_80%_at_100%_0%,rgb(255_122_24/0.25),transparent_60%)] bg-elevated p-3.5">
      <span class="font-display text-xl leading-none font-bold tracking-tight [overflow-wrap:anywhere]">
        {folderTitle(props.name)}
      </span>
    </div>
  );
}

export function Cover(props: { media: MediaSummary | null | undefined; name: string; eager?: boolean; class?: string }) {
  const [loaded, setLoaded] = createSignal(false);
  const src = () => props.media?.coverImage?.extraLarge ?? props.media?.coverImage?.large;
  return (
    <div
      class={cx("relative aspect-[2/3] overflow-hidden rounded-xl bg-surface", props.class)}
      style={{ "background-color": props.media?.coverImage?.color ?? undefined }}
    >
      <Show when={src()} fallback={<TypePoster name={props.name} />}>
        <img
          src={src()!}
          alt=""
          loading={props.eager ? "eager" : "lazy"}
          decoding="async"
          onLoad={() => setLoaded(true)}
          class={cx(
            "size-full object-cover transition duration-500 ease-expo group-hover:scale-[1.04]",
            loaded() ? "opacity-100" : "opacity-0",
          )}
        />
      </Show>
    </div>
  );
}

/** Selo de "está no acervo", sobre a capa. */
export function NuvemBadge() {
  return (
    <span
      class="absolute top-2 left-2 z-10 grid size-7 place-items-center rounded-full bg-accent text-white shadow-lg ring-2 ring-black/40 shadow-black/40"
      title="Disponível no acervo"
    >
      <FolderCheck class="size-3.5" aria-hidden="true" />
      <span class="sr-only">Disponível no acervo</span>
    </span>
  );
}

/** Card de pôster. Com `media` usa direto; com `name`+`type` consulta o AniList ao aparecer na tela. */
export function PosterCard(props: {
  name: string;
  type?: MediaType;
  media?: MediaSummary | null;
  onOpen: () => void;
  subtitle?: string;
  /** Selo sobre a capa (ex.: `NuvemBadge`). */
  badge?: JSX.Element;
  /** Esmaece o card (ex.: título que não está no acervo). */
  dimmed?: boolean;
}) {
  let el!: HTMLButtonElement;
  const [visible, setVisible] = createSignal(false);

  onMount(() => {
    if (props.media !== undefined || !props.type) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setVisible(true);
          io.disconnect();
        }
      },
      { rootMargin: "400px 0px" },
    );
    io.observe(el);
    onCleanup(() => io.disconnect());
  });

  const [looked] = createResource(
    () => (visible() && props.type ? { name: props.name, type: props.type } : undefined),
    ({ name, type }) => lookupMedia(name, type),
  );
  const media = () => (props.media !== undefined ? props.media : looked());
  const loading = () => props.media === undefined && (!visible() || looked.loading);
  const year = () => (media() ? mediaYear(media()!) : folderYear(props.name));
  const title = () => folderTitle(props.name);

  return (
    <button
      ref={el}
      type="button"
      onClick={props.onOpen}
      title={props.name}
      class={cx(
        "group flex w-full flex-col gap-2 rounded-xl text-left outline-offset-4 transition-opacity duration-300",
        props.dimmed && "opacity-55 hover:opacity-100",
      )}
    >
      <div class="relative w-full transition duration-300 ease-expo group-hover:-translate-y-1 group-active:scale-[0.98]">
        {props.badge}
        <Show when={!loading()} fallback={<div class="skeleton aspect-[2/3] rounded-xl" />}>
          <Cover media={media()} name={props.name} class="shadow-lg shadow-black/40 ring-1 ring-white/6" />
        </Show>
        <Show when={media()?.averageScore}>
          <span class="absolute top-2 right-2 flex items-center gap-1 rounded-md bg-black/70 px-1.5 py-0.5 text-xs font-semibold tabular-nums backdrop-blur-sm">
            <Star class="size-3 fill-amber-300 text-amber-300" aria-hidden="true" />
            {media()!.averageScore}
            <span class="sr-only">% de nota média</span>
          </span>
        </Show>
      </div>
      <span class="line-clamp-2 text-sm leading-snug font-semibold group-hover:text-accent-ink">{title()}</span>
      <span class="-mt-1.5 flex gap-2 text-xs text-faint">
        <Show when={props.subtitle} fallback={<>
          <Show when={media()?.format}>
            <span>{label.format(media()!.format)}</span>
          </Show>
          <Show when={year()}>
            <span>{year()}</span>
          </Show>
        </>}>
          <span>{props.subtitle}</span>
        </Show>
      </span>
      <Show when={media() && mediaTitle(media()!) !== title() && !props.subtitle}>
        <span class="-mt-2 truncate text-xs text-muted">{mediaTitle(media()!)}</span>
      </Show>
    </button>
  );
}
