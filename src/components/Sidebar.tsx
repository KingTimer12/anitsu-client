import { House, LogOut } from "lucide-solid";
import { createResource, For, Show } from "solid-js";
import { auth, displayName, nuvem, sessionStore } from "../lib/anitsu";
import { splitPath } from "../lib/library";
import { router } from "../lib/router";
import { sectionMeta, sectionOrder } from "./sections";
import { cx, IconButton } from "./ui";

function NavItem(props: { active: boolean; onClick: () => void; icon: (p: { class?: string }) => any; label: string; title?: string }) {
  return (
    <button
      type="button"
      onClick={props.onClick}
      aria-current={props.active ? "page" : undefined}
      title={props.title ?? props.label}
      class={cx(
        "group relative flex h-10 w-full items-center gap-3 rounded-xl px-3 text-sm font-medium transition-colors duration-150",
        props.active ? "bg-white/8 text-fg" : "text-muted hover:bg-white/5 hover:text-fg",
      )}
    >
      <span
        class={cx(
          "absolute top-2.5 bottom-2.5 left-0 w-[3px] rounded-r-full bg-brand transition-opacity",
          props.active ? "opacity-100" : "opacity-0",
        )}
        aria-hidden="true"
      />
      <props.icon class={cx("size-[18px] shrink-0", props.active ? "text-brand" : "text-faint group-hover:text-muted")} />
      <span class="truncate">{props.label}</span>
    </button>
  );
}

export function Sidebar() {
  const [root] = createResource(() => nuvem.list(""));
  const sections = () =>
    (root()?.files ?? [])
      .filter((f) => f.is_directory)
      .map((f) => f.name)
      .sort(sectionOrder);
  const current = () => splitPath(router.path())[0] ?? "";
  const user = () => sessionStore.session()?.user;

  return (
    <aside class="flex w-60 shrink-0 flex-col border-r border-line bg-bg/60" aria-label="Navegação">
      <nav class="scrollbar-thin flex flex-1 flex-col gap-0.5 overflow-y-auto p-3">
        <NavItem active={current() === ""} onClick={() => router.go("")} icon={House} label="Início" />
        <p class="mt-5 mb-1.5 px-3 text-xs font-medium text-faint">Biblioteca</p>
        <Show
          when={root()}
          fallback={
            <For each={[1, 2, 3, 4, 5]}>{() => <div class="skeleton mx-1 my-1 h-8 rounded-lg" />}</For>
          }
        >
          <For each={sections()}>
            {(name) => {
              const meta = sectionMeta(name);
              return (
                <NavItem
                  active={current() === name}
                  onClick={() => router.go(name)}
                  icon={meta.icon}
                  label={meta.label}
                  title={name}
                />
              );
            }}
          </For>
        </Show>
      </nav>

      <div class="flex items-center gap-3 border-t border-line p-3">
        <Show
          when={user()?.user_metadata?.avatar_url}
          fallback={<span class="size-9 shrink-0 rounded-full bg-white/8" />}
        >
          <img src={user()!.user_metadata!.avatar_url as string} alt="" class="size-9 shrink-0 rounded-full" />
        </Show>
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-semibold">{displayName(user())}</p>
          <p class="text-xs text-faint">Via Discord</p>
        </div>
        <IconButton label="Sair da conta" onClick={() => auth.logout()} class="hover:text-danger">
          <LogOut class="size-4" />
        </IconButton>
      </div>
    </aside>
  );
}
