import { Show } from "solid-js";
import { cx } from "./ui";

/** O AniList usa `.../default.jpg` ("NO IMAGE") quando não há foto. */
const isPlaceholder = (url: string | null | undefined) => !url || /\/default\.(jpg|png)$/i.test(url);

const initials = (name: string | null | undefined) =>
  (name ?? "?")
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((p) => p[0]!.toUpperCase())
    .join("");

/** Foto de pessoa/personagem; monograma na cor do título quando não há imagem. */
export function Portrait(props: { src: string | null | undefined; name: string | null | undefined; class?: string }) {
  return (
    <Show
      when={!isPlaceholder(props.src)}
      fallback={
        <span
          aria-hidden="true"
          class={cx("grid place-items-center bg-accent-soft font-display font-semibold text-accent-ink", props.class)}
        >
          {initials(props.name)}
        </span>
      }
    >
      <img src={props.src!} alt="" loading="lazy" class={cx("object-cover", props.class)} />
    </Show>
  );
}
