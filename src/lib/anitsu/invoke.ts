import { invoke as tauriInvoke, type InvokeArgs } from "@tauri-apps/api/core";
import { AnitsuError } from "./error";

/** `invoke` que converte rejeições do Rust em {@link AnitsuError}. */
export async function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (e) {
    throw AnitsuError.from(e);
  }
}
