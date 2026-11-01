import { ChevronRight } from "lucide-solid";
import { For, Show } from "solid-js";
import { splitPath } from "../lib/library";
import { router } from "../lib/router";
import { sectionMeta } from "./sections";

export function Breadcrumbs(props: { path: string; class?: string }) {
  const segs = () => splitPath(props.path);
  const label = (seg: string, i: number) => (i === 0 ? sectionMeta(seg).label : seg);

  return (
    <nav aria-label="Caminho" class={props.class}>
      <ol class="flex min-w-0 flex-wrap items-center gap-1 text-sm text-muted">
        <li>
          <button type="button" class="rounded-md px-1 hover:text-fg" onClick={() => router.go("")}>
            Início
          </button>
        </li>
        <For each={segs()}>
          {(seg, i) => (
            <li class="flex min-w-0 items-center gap-1">
              <ChevronRight class="size-3.5 shrink-0 text-faint" aria-hidden="true" />
              <Show
                when={i() < segs().length - 1}
                fallback={
                  <span aria-current="page" class="max-w-[42ch] truncate px-1 text-fg">
                    {label(seg, i())}
                  </span>
                }
              >
                <button
                  type="button"
                  class="max-w-[28ch] truncate rounded-md px-1 hover:text-fg"
                  onClick={() => router.go(segs().slice(0, i() + 1).join("/"))}
                >
                  {label(seg, i())}
                </button>
              </Show>
            </li>
          )}
        </For>
      </ol>
    </nav>
  );
}
