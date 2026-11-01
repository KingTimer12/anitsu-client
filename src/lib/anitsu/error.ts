export type AnitsuErrorKind =
  | "not_authenticated"
  | "cancelled"
  | "timeout"
  | "in_progress"
  | "invalid_session"
  | "network"
  | "provider"
  | "rate_limited"
  | "not_found"
  | "anilist"
  | "storage"
  | "http"
  | "decode"
  | "invalid_path"
  | "io"
  | "internal";

/** Erro vindo do Rust (`AuthError` / `HttpError` serializados como `{ kind, message, status? }`). */
export class AnitsuError extends Error {
  constructor(
    readonly kind: AnitsuErrorKind,
    message: string,
    readonly status?: number,
  ) {
    super(message);
    this.name = "AnitsuError";
  }

  static from(e: unknown): AnitsuError {
    if (e instanceof AnitsuError) return e;
    if (e && typeof e === "object" && "kind" in e) {
      const { kind, message, status } = e as { kind: AnitsuErrorKind; message?: string; status?: number | null };
      return new AnitsuError(kind, message ?? kind, status ?? undefined);
    }
    return new AnitsuError("internal", String(e));
  }

  get isAuthError() {
    return this.kind === "not_authenticated" || this.kind === "invalid_session";
  }
}
