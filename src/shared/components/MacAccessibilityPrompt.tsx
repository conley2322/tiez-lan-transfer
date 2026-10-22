import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isMacPlatform } from "../lib/platform";
import { isTauriRuntime } from "../lib/tauriRuntime";
import { requestMacAccessibility } from "../lib/macAccessibility";
import ConfirmDialog from "./ConfirmDialog";

export default function MacAccessibilityPrompt({ ready, theme, t, pushToast }: {
  ready: boolean;
  theme: string;
  t: (key: string) => string;
  pushToast: (message: string, duration?: number) => number;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!ready || !isTauriRuntime() || !isMacPlatform()) return;
    let alive = true;
    let dispose: (() => void) | undefined;
    void invoke<boolean>("check_macos_permissions").then(granted => {
      if (alive && !granted) setOpen(true);
    }).catch(console.error);
    void listen("accessibility-permission-required", () => {
      if (alive) setOpen(true);
    }).then(unlisten => {
      if (alive) dispose = unlisten;
      else unlisten();
    }).catch(console.error);
    return () => { alive = false; dispose?.(); };
  }, [ready]);

  useEffect(() => {
    if (!open) return;
    let alive = true;
    const check = () => {
      void invoke<boolean>("check_macos_permissions").then(granted => {
        if (alive && granted) setOpen(false);
      }).catch(console.error);
    };
    const interval = window.setInterval(check, 3000);
    window.addEventListener("focus", check);
    return () => {
      alive = false;
      window.clearInterval(interval);
      window.removeEventListener("focus", check);
    };
  }, [open]);

  const authorize = async () => {
    setBusy(true);
    try {
      await requestMacAccessibility();
    } catch {
      pushToast(t("accessibility_settings_failed"), 5000);
    } finally {
      setBusy(false);
    }
  };

  return <ConfirmDialog
    open={open}
    title={t("accessibility_permission")}
    message={t("accessibility_permission_prompt")}
    theme={theme}
    busy={busy}
    confirmLabel={t("accessibility_go_authorize")}
    cancelLabel={t("accessibility_permission_later")}
    onConfirm={() => { void authorize(); }}
    onClose={() => setOpen(false)}
  />;
}
