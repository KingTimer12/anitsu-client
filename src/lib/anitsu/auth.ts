import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "./invoke";

export interface User {
  id: string;
  email?: string;
  user_metadata?: {
    full_name?: string;
    name?: string;
    avatar_url?: string;
    custom_claims?: { global_name?: string };
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export interface Session {
  access_token: string;
  refresh_token: string;
  token_type?: string;
  expires_in?: number;
  /** Unix timestamp em segundos. */
  expires_at?: number;
  user?: User;
}

/**
 * Sessão é gerenciada pelo Rust (login via webview, disco, refresh).
 * Para chamar APIs use `AnitsuHttp`/`nuvem`, que já mandam a sessão sozinhos.
 */
export const auth = {
  /**
   * Login pelo navegador padrão (Discord já logado lá). Resolve quando o
   * navegador volta para o app; rejeita com `cancelled`, `timeout` ou `provider`.
   */
  login: () => invoke<Session>("auth_login"),
  /** Reabre a página de login em andamento no navegador. */
  reopenLogin: () => invoke<void>("auth_reopen_login"),
  cancelLogin: () => invoke<void>("auth_cancel_login"),
  logout: () => invoke<void>("auth_logout"),
  /** Sessão válida (renovada se preciso) ou `null`. */
  session: () => invoke<Session | null>("auth_session"),
  refresh: () => invoke<Session>("auth_refresh"),
  onChange: (cb: (session: Session | null) => void): Promise<UnlistenFn> =>
    listen<Session | null>("auth://changed", (e) => cb(e.payload)),
};

export function displayName(user?: User): string | undefined {
  const meta = user?.user_metadata;
  return meta?.custom_claims?.global_name ?? meta?.full_name ?? meta?.name ?? user?.email;
}
