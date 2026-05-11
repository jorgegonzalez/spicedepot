import { Channel, invoke as tauriInvoke } from "@tauri-apps/api/core";

/**
 * Typed wrapper around Tauri `invoke`. The Rust side returns
 * `Result<T, AppError>`; on the Err side Tauri rejects the promise with the
 * serialized error, which we re-throw as a regular Error here so React Query
 * surfaces it.
 */
export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await tauriInvoke<T>(command, args);
  } catch (e) {
    if (typeof e === "string") throw new Error(e);
    if (e instanceof Error) throw e;
    throw new Error(JSON.stringify(e));
  }
}

export { Channel };
