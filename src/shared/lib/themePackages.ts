import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "../types";
import { isTauriRuntime } from "./tauriRuntime";

export interface InstalledThemeDefinition {
  id: string;
  packageId: string;
  name: Partial<Record<Locale, string>>;
  description?: Partial<Record<Locale, string>>;
  category: string;
  version: string;
  minAppVersion: string;
  supportsCustomBackground: boolean;
  supportsSurfaceOpacity: boolean;
  windowMaterial: "none" | "mica" | "acrylic";
}

const STYLE_ID_PREFIX = "tiez-installed-theme-";

export const listInstalledThemes = async (): Promise<InstalledThemeDefinition[]> => {
  if (!isTauriRuntime()) return [];
  return invoke<InstalledThemeDefinition[]>("list_installed_themes");
};

export const getInstalledThemeLabel = (
  theme: InstalledThemeDefinition,
  locale: Locale
): string => theme.name[locale] || theme.name.zh || theme.name.en || theme.packageId;

export const ensureInstalledThemeStyle = async (
  themeId: string,
  options: { force?: boolean } = {}
): Promise<boolean> => {
  if (!themeId.startsWith("store-")) return true;
  if (!isTauriRuntime()) return false;

  const styleId = `${STYLE_ID_PREFIX}${themeId}`;
  const existing = document.getElementById(styleId) as HTMLStyleElement | null;
  if (existing && !options.force) return true;

  try {
    const css = await invoke<string>("get_installed_theme_css", { themeId });
    const style = existing || document.createElement("style");
    style.id = styleId;
    style.dataset.themeId = themeId;
    style.textContent = css;
    if (!existing) document.head.appendChild(style);
    return true;
  } catch (error) {
    console.error(`[Theme] Failed to load installed theme ${themeId}:`, error);
    existing?.remove();
    return false;
  }
};

export const removeInstalledThemeStyle = (themeId: string) => {
  document.getElementById(`${STYLE_ID_PREFIX}${themeId}`)?.remove();
};

