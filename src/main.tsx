import React, { lazy, Suspense } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import "./styles/components/index.css";
import "./styles/themes/load";
import "./styles/platform-windows.css";
import "./styles/platform-macos.css";
import "./styles/components/pin-sticker.css";
import { installBrowserAcceleratorBlocker } from "./shared/lib/blockBrowserAccelerators";
import ErrorBoundary from "./shared/components/ErrorBoundary";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isTauriRuntime } from "./shared/lib/tauriRuntime";

// Suppress Chromium/WebView browser accelerators in every window (main, preview, settings…).
installBrowserAcceleratorBlocker();

const params = new URLSearchParams(window.location.search);
const isCompactPreview = params.get("window") === "compact-preview";
const isPinSticker = params.get("window") === "pin-sticker";
const isCornerBox = params.get("window") === "corner";
const isLanSettings = params.get("window") === "settings";
const isLanFloat = ["progress", "pair", "receive"].includes(params.get("window") ?? "");
const RootComponent = isCompactPreview
  ? lazy(() => import("./features/clipboard/components/CompactPreviewWindow"))
  : isPinSticker
    ? lazy(() => import("./features/clipboard/components/PinStickerWindow"))
    : isCornerBox
      ? lazy(() => import("./features/corner/CornerBoxWindow"))
      : isLanSettings
        ? lazy(() => import("./features/lan/LanSettingsWindow"))
        : isLanFloat
          ? lazy(() => import("./features/lan/LanFloatWindow"))
          : lazy(() => import("./App"));
const isMacPlatform =
  /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent) || /Mac/i.test(navigator.platform);

if (isPinSticker) {
  document.documentElement.classList.add("window-pin-sticker");
  document.body.classList.add("window-pin-sticker");
}

// The drop box is drawn on the desktop, so no theme may paint a background under it.
if (isCornerBox) {
  document.documentElement.classList.add("window-corner");
}

if (isLanSettings) {
  document.documentElement.classList.add("window-settings");
}

// The three desktop cards are transparent like the corner box.
if (isLanFloat) {
  document.documentElement.classList.add("window-float");
  document.body.classList.add("window-float");
}

if (isMacPlatform) {
  document.documentElement.classList.add("platform-macos");
  document.body.classList.add("platform-macos");
} else {
  document.documentElement.classList.add("platform-windows");
  document.body.classList.add("platform-windows");
}

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
root.render(
  <React.StrictMode>
    <ErrorBoundary>
      <Suspense fallback={null}>
        <RootComponent />
      </Suspense>
    </ErrorBoundary>
  </React.StrictMode>,
);

// Light mode is about to drop this page. Unmount first so every effect cleanup
// unregisters its Tauri event listener; Tauri keeps them across reloads
// otherwise. Rust waits for the ack (or gives up after a second).
if (
  !isCompactPreview &&
  !isPinSticker &&
  !isCornerBox &&
  !isLanSettings &&
  !isLanFloat &&
  isTauriRuntime()
) {
  const offRelease = listen("light-mode-release", async () => {
    root.unmount();
    (await offRelease)();
    // Let the unlisten calls from the cleanups go out before Rust navigates.
    await new Promise((resolve) => setTimeout(resolve, 100));
    await invoke("main_ui_released").catch(() => {});
  });
}
