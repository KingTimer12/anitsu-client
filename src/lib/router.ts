import { createRoot, createSignal } from "solid-js";

/** Navegação por caminho da nuvem, com voltar/avançar. */
export const router = createRoot(() => {
  const [stack, setStack] = createSignal<string[]>([""]);
  const [index, setIndex] = createSignal(0);

  const path = () => stack()[index()];

  function go(next: string) {
    if (next === path()) return;
    setStack((s) => [...s.slice(0, index() + 1), next]);
    setIndex((i) => i + 1);
    document.querySelector("main")?.scrollTo({ top: 0 });
  }

  const canBack = () => index() > 0;
  const canForward = () => index() < stack().length - 1;
  const back = () => canBack() && setIndex((i) => i - 1);
  const forward = () => canForward() && setIndex((i) => i + 1);

  window.addEventListener("keydown", (e) => {
    if (e.altKey && e.key === "ArrowLeft") back();
    if (e.altKey && e.key === "ArrowRight") forward();
  });
  window.addEventListener("mouseup", (e) => {
    if (e.button === 3) back();
    if (e.button === 4) forward();
  });

  return { path, go, back, forward, canBack, canForward };
});
