import { invoke } from "./invoke";

/** Serviços conhecidos pelo Rust. Cada um tem base URL e tipo de auth fixos. */
export type Service = "site" | "nuvem" | "supabase";

export type QueryValue = string | number | boolean | null | undefined;

export interface RequestOptions {
  query?: Record<string, QueryValue>;
  body?: unknown;
}

/**
 * Cliente de um serviço do Anitsu. A requisição roda no Rust (reqwest do
 * tauri-plugin-http), que injeta base URL, sessão (cookie ou bearer + apikey)
 * e renova o token em 401. Tokens nunca passam pelo JS.
 *
 * ```ts
 * const nuvemHttp = new AnitsuHttp("nuvem");
 * const listing = await nuvemHttp.get<Listing>("files", { query: { path: "Animes" } });
 * ```
 *
 * Respostas JSON viram objeto; outras viram string. Para arquivos grandes use
 * comandos dedicados (ex.: `nuvem.download`).
 */
export class AnitsuHttp {
  constructor(readonly service: Service) {}

  request<T = unknown>(method: string, path: string, options: RequestOptions = {}): Promise<T> {
    return invoke<T>("anitsu_request", {
      request: { service: this.service, method, path, query: options.query, body: options.body },
    });
  }

  get<T = unknown>(path: string, options?: Omit<RequestOptions, "body">) {
    return this.request<T>("GET", path, options);
  }

  post<T = unknown>(path: string, body?: unknown, options?: Omit<RequestOptions, "body">) {
    return this.request<T>("POST", path, { ...options, body });
  }

  put<T = unknown>(path: string, body?: unknown, options?: Omit<RequestOptions, "body">) {
    return this.request<T>("PUT", path, { ...options, body });
  }

  patch<T = unknown>(path: string, body?: unknown, options?: Omit<RequestOptions, "body">) {
    return this.request<T>("PATCH", path, { ...options, body });
  }

  delete<T = unknown>(path: string, options?: RequestOptions) {
    return this.request<T>("DELETE", path, options);
  }
}

/** `https://anitsu.moe/` (cookie). */
export const siteHttp = new AnitsuHttp("site");
/** `https://nuvem.anitsu.moe/api/` (cookie). */
export const nuvemHttp = new AnitsuHttp("nuvem");
/** `https://auth.anitsu.moe/rest/v1/` (apikey + bearer). */
export const supabaseHttp = new AnitsuHttp("supabase");
