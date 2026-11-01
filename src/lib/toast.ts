import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";

export type ToastTone = "info" | "ok" | "error";

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
}

const DURATION_MS = 4000;

/** Avisos curtos, sem roubar foco (região aria-live no Toaster). */
export const toast = createRoot(() => {
  const [items, setItems] = createStore<Toast[]>([]);
  let next = 1;

  function dismiss(id: number) {
    setItems((list) => list.filter((t) => t.id !== id));
  }

  function show(message: string, tone: ToastTone = "info") {
    const id = next++;
    setItems((list) => [...list.slice(-2), { id, message, tone }]);
    setTimeout(() => dismiss(id), DURATION_MS);
  }

  return { items, show, dismiss };
});
