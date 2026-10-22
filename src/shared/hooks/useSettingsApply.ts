import { getNativePaletteStyle, normalizeNativePalette } from "../config/nativePalettes";
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyNativePalette, applyThemeClasses, normalizeThemeId } from "../config/themes";
import { DEFAULT_THEME } from "../config/themes";
import { ensureInstalledThemeStyle } from "../lib/themePackages";

interface UseSettingsApplyOptions {
  theme: string;
  colorMode: string;
  nativePalette: string;

  compactMode: boolean;
  compactShowTime: boolean;
  hideUnselectedItemActions: boolean;
  settingsLoaded: boolean;
  clipboardItemFontSize: number;
  clipboardTagFontSize: number;
  surfaceOpacity: number;
  showAppBorder: boolean;
}

export const useSettingsApply = ({
  theme,
  colorMode,
  nativePalette,

  compactMode,
  compactShowTime,
  hideUnselectedItemActions,
  settingsLoaded,
  clipboardItemFontSize,
  clipboardTagFontSize,
  surfaceOpacity,
  showAppBorder
}: UseSettingsApplyOptions) => {
  useEffect(() => {
    if (!settingsLoaded) return;

    const root = document.documentElement;
    const body = document.body;

    let disposed = false;
    const normalizedTheme = normalizeThemeId(theme);

    const applyExplicitMode = (mode: "light" | "dark") => {
      if (disposed) return;
      root.classList.remove("light-mode", "dark-mode");
      body.classList.remove("light-mode", "dark-mode");
      if (mode === "dark") {
        root.classList.add("dark-mode");
        body.classList.add("dark-mode");
      } else {
        root.classList.add("light-mode");
        body.classList.add("light-mode");
      }
    };

    const applySystemMode = async () => {
      try {
        const current = await getCurrentWindow().theme();
        if (disposed) return;
        applyExplicitMode(current === "dark" ? "dark" : "light");
      } catch {
        if (disposed) return;
        const isDark =
          window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
        applyExplicitMode(isDark ? "dark" : "light");
      }
    };

    const applySelectedTheme = async () => {
      const installedThemeReady = await ensureInstalledThemeStyle(normalizedTheme);
      if (disposed) return;
      const effectiveTheme = installedThemeReady ? normalizedTheme : DEFAULT_THEME;
      applyThemeClasses(effectiveTheme, root, body);
      invoke("set_theme", {
        theme: effectiveTheme,
        colorMode,
        showAppBorder,
      }).catch(console.error);
    };

    void applySelectedTheme();

    if (compactMode) {
      body.classList.add("compact-mode");
    } else {
      body.classList.remove("compact-mode");
    }

    body.classList.toggle("hide-unselected-item-actions", hideUnselectedItemActions);
    body.classList.toggle("compact-show-time", compactShowTime);
    root.classList.toggle("compact-show-time", compactShowTime);

    if (colorMode === "light") {
      applyExplicitMode("light");
    } else if (colorMode === "dark") {
      applyExplicitMode("dark");
    } else {
      applySystemMode();
    }

    let unlisten: (() => void) | null = null;
    let cleanupMedia: (() => void) | null = null;

    if (colorMode === "system") {
      getCurrentWindow()
        .onThemeChanged((event) => {
          if (disposed) return;
          const next = event?.payload === "dark" ? "dark" : "light";
          applyExplicitMode(next);
          void applySelectedTheme();
        })
        .then((f) => {
          if (disposed) {
            f();
            return;
          }
          unlisten = f;
        });

      if (window.matchMedia) {
        const media = window.matchMedia("(prefers-color-scheme: dark)");
        const onChange = () => applyExplicitMode(media.matches ? "dark" : "light");
        if (media.addEventListener) {
          media.addEventListener("change", onChange);
          cleanupMedia = () => media.removeEventListener("change", onChange);
        } else {
          media.addListener(onChange);
          cleanupMedia = () => media.removeListener(onChange);
        }
      }
    }

    return () => {
      disposed = true;
      if (unlisten) unlisten();
      if (cleanupMedia) cleanupMedia();
    };
  }, [theme, colorMode, showAppBorder, settingsLoaded, compactMode, compactShowTime, hideUnselectedItemActions]);

  useEffect(() => {
    if (!settingsLoaded) return;
    applyNativePalette(theme, nativePalette, document.documentElement, document.body);
    try {
      localStorage.setItem("tiez_native_palette_tokens", JSON.stringify({
        palette: normalizeNativePalette(nativePalette), styles: getNativePaletteStyle(nativePalette)
      }));
    } catch { /* The database remains authoritative if the startup cache is unavailable. */ }
  }, [theme, nativePalette, settingsLoaded]);

  useEffect(() => {
    if (!settingsLoaded) return;
    const root = document.documentElement;
    root.style.setProperty("--clipboard-item-font-size", `${clipboardItemFontSize}px`);
    root.style.setProperty("--clipboard-tag-font-size", `${clipboardTagFontSize}px`);
    const safeSurfaceOpacity = Number.isFinite(surfaceOpacity)
      ? Math.min(100, Math.max(0, surfaceOpacity))
      : 50;
    const scale = Math.min(2, Math.max(0, safeSurfaceOpacity / 50));
    root.style.setProperty("--surface-opacity-scale", scale.toString());
  }, [clipboardItemFontSize, clipboardTagFontSize, surfaceOpacity, settingsLoaded]);
};
