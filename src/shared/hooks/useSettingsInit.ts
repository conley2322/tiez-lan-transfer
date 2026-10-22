import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { message } from "@tauri-apps/plugin-dialog";
import { DEFAULT_THEME, migrateThemeSettings, resolveThemeAppearance } from "../config/themes";
import type { Locale } from "../types";
import { isTauriRuntime } from "../lib/tauriRuntime";
import { formatError } from "../lib/errorMessage";

interface UseSettingsInitOptions {
  setAppSettings: (settings: Record<string, string>) => void;
  setHotkey: (val: string) => void;
  setTheme: (val: string) => void;
  setColorMode: (val: string) => void;
  setNativePalette: (val: string) => void;
  setCompactMode: (val: boolean) => void;
  setLanguage: (val: Locale) => void;
}

export const useSettingsInit = ({
  setAppSettings,
  setHotkey,
  setTheme,
  setColorMode,
  setNativePalette,
  setCompactMode,
  setLanguage
}: UseSettingsInitOptions) => {
  const [settings, setSettings] = useState<Record<string, string> | null>(null);
  const settingsEffectCount = useRef(0);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;

    const loadSettings = () => {
      settingsEffectCount.current++;
      console.log(`[THEME DEBUG] Settings useEffect run #${settingsEffectCount.current}`);

      invoke<Record<string, string>>("get_settings")
        .then((result) => {
          if (disposed) return;

          console.log("[THEME DEBUG] app.color_mode from DB:", result["app.color_mode"]);

          const migrated = migrateThemeSettings(result);
          if (migrated !== result) {
            // Save the palette first so an interrupted migration can safely retry.
            void invoke("save_setting", { key: "app.native_palette", value: "sakura" })
              .then(() => invoke("save_setting", { key: "app.theme", value: migrated["app.theme"] }))
              .catch(error => console.error("Theme migration failed", error));
          }
          setAppSettings(migrated);
          // Empty: the user cleared it (as for the other hotkeys).
          if (result["app.hotkey"] !== undefined) setHotkey(result["app.hotkey"]);

          const { theme: loadedTheme, nativePalette } = resolveThemeAppearance(migrated["app.theme"] || DEFAULT_THEME, migrated["app.native_palette"]);
          const loadedColorMode = result["app.color_mode"] || "system";
          console.log("[THEME DEBUG] loadedColorMode:", loadedColorMode);

          setTheme(loadedTheme);
          setNativePalette(nativePalette);
          setColorMode(loadedColorMode);
          setCompactMode(result["app.compact_mode"] === "true");

          try {
            localStorage.setItem("tiez_theme", loadedTheme);
            localStorage.setItem("tiez_native_palette", nativePalette);
            localStorage.setItem("tiez_color_mode", loadedColorMode);
            localStorage.setItem(
              "tiez_compact_mode",
              result["app.compact_mode"] === "true" ? "true" : "false"
            );
          } catch {
            // Ignore localStorage errors
          }

          if (result["app.language"]) {
            setLanguage(result["app.language"] as Locale);
            try {
              localStorage.setItem("tiez_language", result["app.language"]);
            } catch {
              // Ignore localStorage errors
            }
          }

          setSettings(migrated);
        })
        .catch((error) => {
          if (disposed) return;
          console.error("Settings initialization failed", error);
          void message("设置读取失败，原数据已保留。\n请检查原设备密钥或钥匙串权限。\n然后重新启动应用。\n" + formatError(error), { title: "设置异常", kind: "error" });
        });
    };

    loadSettings();

    const unlisten = listen("settings-changed", () => {
      loadSettings();
    });

    return () => {
      disposed = true;
      unlisten.then((off) => off());
    };
  }, [setAppSettings, setHotkey, setTheme, setColorMode, setNativePalette, setCompactMode, setLanguage]);

  return settings;
};
