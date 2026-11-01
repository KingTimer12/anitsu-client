import { createRoot } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { AnitsuError, nuvem } from "./anitsu";
import { toast } from "./toast";

export type DownloadStatus = "queued" | "running" | "done" | "error";

export interface Download {
  id: string;
  path: string;
  name: string;
  downloaded: number;
  total: number | null;
  status: DownloadStatus;
  dest?: string;
  error?: string;
}

/** Arquivos da nuvem têm GBs; poucos ao mesmo tempo rendem mais que muitos. */
const MAX_CONCURRENT = 2;

/** Downloads da sessão, em fila, atualizados pelo evento de progresso do Rust. */
export const downloads = createRoot(() => {
  const [items, setItems] = createStore<Download[]>([]);

  void nuvem.onDownloadProgress((p) => {
    setItems(
      (d) => d.id === p.id,
      produce((d) => {
        d.downloaded = p.downloaded;
        d.total = p.total;
      }),
    );
  });

  function pump() {
    let running = items.filter((d) => d.status === "running").length;
    for (const d of items.slice().reverse()) {
      if (running >= MAX_CONCURRENT) break;
      if (d.status !== "queued") continue;
      running++;
      run(d.id, d.path);
    }
  }

  function run(id: string, path: string) {
    setItems((d) => d.id === id, { status: "running", error: undefined, downloaded: 0 });
    nuvem
      .download(path, { id })
      .then((dest) => {
        setItems((d) => d.id === id, { status: "done", dest });
        const name = items.find((d) => d.id === id)?.name;
        toast.show(`${name ?? "Arquivo"} baixado`, "ok");
      })
      .catch((e) => {
        setItems((d) => d.id === id, { status: "error", error: AnitsuError.from(e).message });
        toast.show("Um download falhou. Abra Downloads para tentar de novo.", "error");
      })
      .finally(pump);
  }

  function enqueue(path: string) {
    const existing = items.find((d) => d.path === path);
    if (existing && (existing.status === "running" || existing.status === "queued")) return;
    if (existing) {
      setItems((d) => d.id === existing.id, { status: "queued", error: undefined, downloaded: 0 });
    } else {
      const name = path.split("/").pop() ?? path;
      setItems((list) => [
        { id: crypto.randomUUID(), path, name, downloaded: 0, total: null, status: "queued" },
        ...list,
      ]);
    }
  }

  function start(path: string) {
    enqueue(path);
    pump();
  }

  function startAll(paths: string[]) {
    paths.forEach(enqueue);
    pump();
  }

  const byPath = (path: string) => items.find((d) => d.path === path);
  const pending = () => items.filter((d) => d.status === "running" || d.status === "queued").length;
  /** Progresso agregado dos downloads ativos (0–1), para o indicador da barra de título. */
  const overall = () => {
    const active = items.filter((d) => d.status === "running" && d.total);
    const total = active.reduce((s, d) => s + (d.total ?? 0), 0);
    return total ? active.reduce((s, d) => s + d.downloaded, 0) / total : 0;
  };
  const clearFinished = () => setItems((list) => list.filter((d) => d.status === "running" || d.status === "queued"));

  return { items, start, startAll, byPath, pending, overall, clearFinished };
});

export const progressOf = (d: Pick<Download, "downloaded" | "total">) =>
  d.total ? Math.min(1, d.downloaded / d.total) : 0;
