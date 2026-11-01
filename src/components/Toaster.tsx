import { Check, Info, TriangleAlert, X } from "lucide-solid";
import { For, Match, Switch } from "solid-js";
import { toast } from "../lib/toast";
import { cx } from "./ui";

export function Toaster() {
  return (
    <div aria-live="polite" class="pointer-events-none fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 flex-col items-center gap-2">
      <For each={toast.items}>
        {(t) => (
          <div
            role="status"
            class="pointer-events-auto flex animate-fade-up items-center gap-3 rounded-xl border border-line-strong bg-elevated/95 py-2.5 pr-2 pl-3.5 text-sm shadow-xl shadow-black/50 backdrop-blur-xl"
          >
            <Switch>
              <Match when={t.tone === "ok"}>
                <Check class="size-4 text-ok" aria-hidden="true" />
              </Match>
              <Match when={t.tone === "error"}>
                <TriangleAlert class="size-4 text-danger" aria-hidden="true" />
              </Match>
              <Match when={true}>
                <Info class="size-4 text-muted" aria-hidden="true" />
              </Match>
            </Switch>
            <span class={cx("max-w-[60ch]", t.tone === "error" && "text-fg")}>{t.message}</span>
            <button
              type="button"
              aria-label="Fechar aviso"
              onClick={() => toast.dismiss(t.id)}
              class="rounded-md p-1 text-faint hover:bg-white/8 hover:text-fg"
            >
              <X class="size-3.5" />
            </button>
          </div>
        )}
      </For>
    </div>
  );
}
