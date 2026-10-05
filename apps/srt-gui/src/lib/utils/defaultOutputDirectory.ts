import { downloadDir, homeDir } from "@tauri-apps/api/path";
import { invoke } from "@tauri-apps/api/core";

/** Use an existing OS Downloads folder, falling back to the user's home directory. */
export async function defaultOutputDirectory(): Promise<string> {
  try {
    const path = await downloadDir();
    if (await invoke<boolean>("flashcard_check_dir_exists", { path })) return path;
  } catch { /* Some platforms cannot resolve the Downloads directory. */ }
  return await homeDir();
}
