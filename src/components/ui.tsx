import { LoaderCircle } from "lucide-solid";
import { splitProps, type JSX } from "solid-js";

const cx = (...parts: (string | false | null | undefined)[]) => parts.filter(Boolean).join(" ");
export { cx };

/** Botão só com ícone: `label` vira aria-label e tooltip. */
export function IconButton(
  props: JSX.ButtonHTMLAttributes<HTMLButtonElement> & { label: string; children: JSX.Element },
) {
  const [local, rest] = splitProps(props, ["label", "children", "class"]);
  return (
    <button
      type="button"
      aria-label={local.label}
      title={local.label}
      class={cx(
        "inline-flex size-9 shrink-0 items-center justify-center rounded-lg text-muted transition duration-150 ease-expo",
        "hover:bg-white/8 hover:text-fg active:scale-95 disabled:pointer-events-none disabled:opacity-35",
        local.class,
      )}
      {...rest}
    >
      {local.children}
    </button>
  );
}

type ButtonVariant = "primary" | "secondary" | "ghost";

export function Button(
  props: JSX.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; loading?: boolean },
) {
  const [local, rest] = splitProps(props, ["variant", "loading", "class", "children", "disabled"]);
  const variant = () => local.variant ?? "secondary";
  return (
    <button
      type="button"
      disabled={local.disabled || local.loading}
      aria-busy={local.loading}
      class={cx(
        "inline-flex h-10 items-center justify-center gap-2 rounded-xl px-4 text-sm font-semibold whitespace-nowrap",
        "transition duration-200 ease-expo active:scale-[0.97] disabled:pointer-events-none disabled:opacity-50",
        variant() === "primary" &&
          "bg-accent text-white shadow-[0_8px_30px_-8px_var(--accent-glow)] hover:brightness-110",
        variant() === "secondary" && "border border-line-strong bg-white/5 text-fg hover:bg-white/10",
        variant() === "ghost" && "text-muted hover:bg-white/8 hover:text-fg",
        local.class,
      )}
      {...rest}
    >
      {local.loading ? <LoaderCircle class="size-4 animate-spin" aria-hidden="true" /> : null}
      {local.children}
    </button>
  );
}

/** Anel de progresso (0–1) com conteúdo no centro. */
export function ProgressRing(props: { value: number; size?: number; stroke?: number; children?: JSX.Element; label?: string }) {
  const size = () => props.size ?? 36;
  const stroke = () => props.stroke ?? 3;
  const r = () => (size() - stroke()) / 2;
  const c = () => 2 * Math.PI * r();
  return (
    <span
      class="relative inline-grid shrink-0 place-items-center"
      style={{ width: `${size()}px`, height: `${size()}px` }}
      role="progressbar"
      aria-label={props.label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(props.value * 100)}
    >
      <svg class="absolute inset-0 -rotate-90" width={size()} height={size()} aria-hidden="true">
        <circle cx={size() / 2} cy={size() / 2} r={r()} fill="none" stroke="currentColor" stroke-opacity="0.15" stroke-width={stroke()} />
        <circle
          cx={size() / 2}
          cy={size() / 2}
          r={r()}
          fill="none"
          stroke="var(--accent-ink)"
          stroke-width={stroke()}
          stroke-linecap="round"
          stroke-dasharray={String(c())}
          stroke-dashoffset={String(c() * (1 - Math.min(1, Math.max(0, props.value))))}
          class="transition-[stroke-dashoffset] duration-300"
        />
      </svg>
      {props.children}
    </span>
  );
}

export function SectionTitle(props: { id: string; children: JSX.Element; aside?: JSX.Element }) {
  return (
    <div class="mb-4 flex items-end justify-between gap-4">
      <h2 id={props.id} class="font-display text-xl font-semibold tracking-tight">
        {props.children}
      </h2>
      {props.aside}
    </div>
  );
}
