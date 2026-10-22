import { invoke } from "@tauri-apps/api/core";

let systemPromptRequested = false;

/**
 * "Authorize" for the macOS Accessibility permission.
 *
 * The first time, only macOS's own prompt is shown: it adds TieZ to the
 * Accessibility list and has its own "Open System Settings" button. Opening
 * Settings at the same time put the Settings window over that prompt before
 * the user could answer it. macOS shows the prompt once; if no prompt took
 * focus, or on later clicks, Settings is opened directly.
 */
export const requestMacAccessibility = async (): Promise<void> => {
  if (!systemPromptRequested) {
    systemPromptRequested = true;
    const granted = await invoke<boolean>("request_macos_permissions");
    if (granted) return;
    await new Promise((resolve) => setTimeout(resolve, 800));
    if (!document.hasFocus()) return; // the system prompt is up
  }
  await invoke("open_macos_accessibility_settings");
};
