import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauriRuntime } from "../lib/tauriRuntime";

interface UseWindowPinnedListenerOptions {
  onPinnedChange: (pinned: boolean) => void;
}

export const useWindowPinnedListener = ({ onPinnedChange }: UseWindowPinnedListenerOptions) => {
  useEffect(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        const nextUnlisten = await listen<boolean>("window-pinned-changed", (event) => {
          const pinned = event.payload === true;
          onPinnedChange(pinned);
        });
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      } catch (e) {
        console.error(e);
      }
    })();

    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, [onPinnedChange]);
};
