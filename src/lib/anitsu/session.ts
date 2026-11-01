import { createRoot, createSignal } from "solid-js";
import { auth, type Session } from "./auth";

/**
 * Estado global e reativo da sessão (Solid). Sincroniza com o Rust
 * pelo evento `auth://changed`.
 */
export const sessionStore = createRoot(() => {
  const [session, setSession] = createSignal<Session | null>(null);
  const [ready, setReady] = createSignal(false);

  void auth.onChange(setSession);
  auth
    .session()
    .then(setSession)
    .catch(() => setSession(null))
    .finally(() => setReady(true));

  return { session, ready, isLoggedIn: () => session() !== null };
});
