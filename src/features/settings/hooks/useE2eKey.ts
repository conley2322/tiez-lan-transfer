import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../../../shared/lib/tauriRuntime";

const CHANGED_EVENT = "tiez:e2e-key-changed";

/** The end-to-end key (docs/E2E-ENCRYPTION.md); "" when encryption is off. */
export const useE2eKey = () => {
  const [key, setKey] = useState("");

  useEffect(() => {
    let alive = true;
    if (isTauriRuntime()) {
      invoke<Record<string, string>>("get_settings")
        .then((settings) => {
          if (alive) setKey(settings["e2e_key"] ?? "");
        })
        .catch(() => {});
    }
    const onChange = (event: Event) => setKey((event as CustomEvent<string>).detail);
    window.addEventListener(CHANGED_EVENT, onChange);
    return () => {
      alive = false;
      window.removeEventListener(CHANGED_EVENT, onChange);
    };
  }, []);

  const save = useCallback(async (value: string) => {
    await invoke("save_setting", { key: "e2e_key", value });
    window.dispatchEvent(new CustomEvent(CHANGED_EVENT, { detail: value }));
  }, []);

  return { key, save };
};
