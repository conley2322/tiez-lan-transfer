import React, { useState, useEffect, useRef, useCallback } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, PhysicalPosition } from "@tauri-apps/api/window";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import {
  Copy,
  Download,
  RotateCw,
  FlipHorizontal,
  Pin,
  PinOff,
  RotateCcw,
  X,
  Check,
} from "lucide-react";
import { translations } from "../../../locales";
import { applyNativePalette, applyThemeClasses, DEFAULT_THEME, normalizeThemeId } from "../../../shared/config/themes";
import type { Locale } from "../../../shared/types";
import { assetUrlToFilePath } from "../../../shared/lib/localImageSrc";

const STICKER_PADDING = 10; // 10px macOS-grade ambient shadow breathing space

export const PinStickerWindow: React.FC = () => {
  useEffect(() => {
    try {
      const theme = localStorage.getItem("tiez_theme") || DEFAULT_THEME;
      const colorMode = localStorage.getItem("tiez_color_mode") || "system";
      applyThemeClasses(normalizeThemeId(theme), document.documentElement, document.body);
      applyNativePalette(theme, localStorage.getItem("tiez_native_palette") || "default", document.documentElement, document.body);

      if (
        colorMode === "dark" ||
        (colorMode === "system" &&
          window.matchMedia("(prefers-color-scheme: dark)").matches)
      ) {
        document.body.classList.add("dark-mode");
        document.body.classList.remove("light-mode");
        document.documentElement.classList.add("dark-mode");
        document.documentElement.classList.remove("light-mode");
      } else {
        document.body.classList.add("light-mode");
        document.body.classList.remove("dark-mode");
        document.documentElement.classList.add("light-mode");
        document.documentElement.classList.remove("dark-mode");
      }
    } catch (e) {
      console.warn("Failed to apply theme in pin sticker:", e);
    }
  }, []);

  // Mirrored from settings "app.language" by the main window (App saveAppSetting / useSettingsInit).
  const storedLanguage = localStorage.getItem("tiez_language");
  const language: Locale = storedLanguage === "en" || storedLanguage === "tw" ? storedLanguage : "zh";
  const t = useCallback(
    (key: keyof (typeof translations)["zh"]) => {
      return (translations[language] && (translations[language] as any)[key]) || (translations.zh as any)[key] || key;
    },
    [language]
  );
  const params = new URLSearchParams(window.location.search);
  const rawSrc = params.get("src") || "";

  const [rotation, setRotation] = useState<number>(0);
  const [flipped, setFlipped] = useState<boolean>(false);
  const [opacity, setOpacity] = useState<number>(1.0);
  const [isAlwaysOnTop, setIsAlwaysOnTop] = useState<boolean>(true);
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [copied, setCopied] = useState<boolean>(false);
  const [imageError, setImageError] = useState<boolean>(false);

  const toastTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const naturalDimsRef = useRef<{ w: number; h: number }>({ w: 400, h: 300 });
  const currentLogicalSizeRef = useRef<{ w: number; h: number }>({ w: 400, h: 300 });
  const hasResizedRef = useRef<boolean>(false);

  const showToast = useCallback((msg: string) => {
    if (toastTimerRef.current) {
      clearTimeout(toastTimerRef.current);
    }
    setToastMessage(msg);
    toastTimerRef.current = setTimeout(() => {
      setToastMessage(null);
    }, 1200);
  }, []);

  // Initial load: Set window size to match image aspect ratio + shadow padding
  const handleImageLoad = useCallback(async (e: React.SyntheticEvent<HTMLImageElement>) => {
    if (hasResizedRef.current) return;
    const img = e.currentTarget;
    const natW = img.naturalWidth;
    const natH = img.naturalHeight;
    if (natW > 0 && natH > 0) {
      hasResizedRef.current = true;
      naturalDimsRef.current = { w: natW, h: natH };

      // Fit nicely on screen if larger than max bounds
      const maxW = 960;
      const maxH = 720;
      let targetW = natW;
      let targetH = natH;
      if (targetW > maxW || targetH > maxH) {
        const scale = Math.min(maxW / targetW, maxH / targetH);
        targetW = Math.round(targetW * scale);
        targetH = Math.round(targetH * scale);
      }
      targetW = Math.max(targetW, 100);
      targetH = Math.max(targetH, 80);

      currentLogicalSizeRef.current = { w: targetW, h: targetH };

      try {
        const win = getCurrentWebviewWindow();
        await win.setSize(new LogicalSize(targetW + STICKER_PADDING * 2, targetH + STICKER_PADDING * 2));
      } catch (err) {
        console.warn("Could not auto-resize sticker window:", err);
      }
    }
  }, []);

  const handleClose = useCallback(async () => {
    try {
      const currentWindow = getCurrentWebviewWindow();
      await currentWindow.close();
    } catch (err) {
      console.error("Failed to close pin sticker window:", err);
    }
  }, []);

  const handleToggleAlwaysOnTop = useCallback(async () => {
    try {
      const currentWindow = getCurrentWebviewWindow();
      const nextState = !isAlwaysOnTop;
      await currentWindow.setAlwaysOnTop(nextState);
      setIsAlwaysOnTop(nextState);
      showToast(nextState ? t("always_on_top") : t("unpin"));
    } catch (err) {
      console.error("Failed to set always on top:", err);
    }
  }, [isAlwaysOnTop, showToast, t]);

  const handleReset = useCallback(async () => {
    setRotation(0);
    setFlipped(false);
    setOpacity(1.0);
    const { w, h } = naturalDimsRef.current;
    if (w > 0 && h > 0) {
      const maxW = 960;
      const maxH = 720;
      let targetW = w;
      let targetH = h;
      if (targetW > maxW || targetH > maxH) {
        const scale = Math.min(maxW / targetW, maxH / targetH);
        targetW = Math.round(targetW * scale);
        targetH = Math.round(targetH * scale);
      }
      currentLogicalSizeRef.current = { w: targetW, h: targetH };
      try {
        const win = getCurrentWebviewWindow();
        await win.setSize(new LogicalSize(targetW + STICKER_PADDING * 2, targetH + STICKER_PADDING * 2));
      } catch (err) {
        console.warn("Failed to reset window size:", err);
      }
    }
    showToast("100%");
  }, [showToast]);

  const handleRotate = useCallback(async () => {
    setRotation((prev) => (prev + 90) % 360);
    try {
      const win = getCurrentWebviewWindow();
      const { w, h } = currentLogicalSizeRef.current;
      // Swap width and height for 90-degree rotations
      const newW = h;
      const newH = w;
      currentLogicalSizeRef.current = { w: newW, h: newH };
      await win.setSize(new LogicalSize(newW + STICKER_PADDING * 2, newH + STICKER_PADDING * 2));
    } catch (err) {
      console.warn("Failed to rotate window:", err);
    }
  }, []);

  const handleCopyImage = useCallback(async () => {
    if (!imgRef.current) return;
    try {
      const canvas = document.createElement("canvas");
      const img = imgRef.current;
      canvas.width = img.naturalWidth || img.width;
      canvas.height = img.naturalHeight || img.height;
      const ctx = canvas.getContext("2d");
      if (ctx) {
        ctx.drawImage(img, 0, 0);
        canvas.toBlob(async (blob) => {
          if (blob) {
            await navigator.clipboard.write([
              new ClipboardItem({ "image/png": blob }),
            ]);
            setCopied(true);
            showToast(t("sticker_copied"));
            setTimeout(() => setCopied(false), 1500);
          }
        }, "image/png");
      }
    } catch (err) {
      console.error("Failed to copy image to clipboard:", err);
    }
  }, [showToast, t]);

  const handleSaveImage = useCallback(async () => {
    try {
      const targetPath = await saveDialog({
        defaultPath: `sticker_${Date.now()}.png`,
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });

      if (!targetPath) return;

      // Sticker images are local files shown through an asset URL, whose form
      // differs per platform (asset://localhost/… vs http://asset.localhost/…
      // on Windows). Copy the original file to the chosen path.
      const sourcePath = assetUrlToFilePath(rawSrc);
      if (!sourcePath) {
        // No backend command writes arbitrary bytes; don't claim a save that
        // did not happen.
        showToast(t("save_image_failed"));
        return;
      }
      await invoke("save_file_copy", { sourcePath, targetPath });
      showToast(t("save_image_success"));
    } catch (err) {
      console.error("Failed to save image:", err);
      showToast(t("save_image_failed"));
    }
  }, [rawSrc, showToast, t]);

  // Handle Wheel: Shift+Wheel for Opacity, Wheel for Proportional Window Zoom
  const handleWheel = useCallback(async (e: React.WheelEvent) => {
    e.preventDefault();
    if (e.shiftKey) {
      // Shift + Scroll: Opacity
      const delta = e.deltaY > 0 ? -0.05 : 0.05;
      setOpacity((prev) => {
        const next = Math.min(Math.max(Number((prev + delta).toFixed(2)), 0.2), 1.0);
        showToast(`${Math.round(next * 100)}%`);
        return next;
      });
      return;
    }

    // Normal Scroll: Strict Proportional Window Scale (Centered)
    const factor = e.deltaY < 0 ? 1.08 : 0.925;
    const { w: curW, h: curH } = currentLogicalSizeRef.current;
    const natW = naturalDimsRef.current.w || 400;
    const natH = naturalDimsRef.current.h || 300;
    const isRotated90 = (rotation / 90) % 2 !== 0;
    const ar = isRotated90 ? natH / natW : natW / natH;

    let newW = Math.round(curW * factor);
    newW = Math.min(Math.max(newW, 80), 2560);
    const newH = Math.round(newW / ar);

    if (newW === curW && newH === curH) return;

    currentLogicalSizeRef.current = { w: newW, h: newH };

    try {
      const win = getCurrentWebviewWindow();
      const currentPos = await win.innerPosition();
      const scaleFactor = await win.scaleFactor();

      const deltaPixelW = Math.round((newW - curW) * scaleFactor);
      const deltaPixelH = Math.round((newH - curH) * scaleFactor);

      const newX = currentPos.x - Math.round(deltaPixelW / 2);
      const newY = currentPos.y - Math.round(deltaPixelH / 2);

      await win.setSize(new LogicalSize(newW + STICKER_PADDING * 2, newH + STICKER_PADDING * 2));
      await win.setPosition(new PhysicalPosition(newX, newY));

      const zoomPercent = Math.round((newW / (isRotated90 ? natH : natW)) * 100);
      showToast(`${zoomPercent}%`);
    } catch (err) {
      console.warn("Failed to proportionally resize window on wheel:", err);
    }
  }, [rotation, showToast]);

  // Window drag handler
  const handleMouseDown = async (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest("button") || target.closest(".pin-sticker-toolbar")) {
      return;
    }
    try {
      const currentWindow = getCurrentWebviewWindow();
      await currentWindow.startDragging();
    } catch (err) {
      console.error("Failed to start dragging sticker:", err);
    }
  };

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        handleClose();
      } else if (e.key === " " || e.code === "Space") {
        e.preventDefault();
        handleReset();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "c") {
        e.preventDefault();
        handleCopyImage();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        handleSaveImage();
      } else if (e.key === "1") {
        setOpacity(0.25);
        showToast("25%");
      } else if (e.key === "2") {
        setOpacity(0.5);
        showToast("50%");
      } else if (e.key === "3") {
        setOpacity(0.75);
        showToast("75%");
      } else if (e.key === "4") {
        setOpacity(1.0);
        showToast("100%");
      } else if (e.key === "r" || e.key === "R") {
        handleRotate();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleClose, handleCopyImage, handleReset, handleRotate, handleSaveImage, showToast, handleRotate]);

  return (
    <div
      className="pin-sticker-root"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onDoubleClick={(e) => {
        const target = e.target as HTMLElement;
        if (!target.closest("button") && !target.closest(".pin-sticker-toolbar")) {
          handleClose();
        }
      }}
    >
      <div
        className={`pin-sticker-shell ${isAlwaysOnTop ? "always-on-top" : ""}`}
        style={{ opacity }}
      >
        {/* Floating Toast Notification */}
        {toastMessage && <div className="pin-sticker-toast">{toastMessage}</div>}

        {/* Floating Action Toolbar on BOTTOM Center of Image (Upper Layer) */}
        <div className="pin-sticker-toolbar" onMouseDown={(e) => e.stopPropagation()}>
          <button
            type="button"
            className="pin-sticker-btn"
            onClick={handleCopyImage}
            title={t("copy")}
          >
            {copied ? <Check size={14} style={{ color: "var(--status-success)" }} /> : <Copy size={14} />}
          </button>

          <button
            type="button"
            className="pin-sticker-btn"
            onClick={handleSaveImage}
            title={t("save_image")}
          >
            <Download size={14} />
          </button>

          <div className="pin-sticker-divider" />

          <button
            type="button"
            className="pin-sticker-btn"
            onClick={handleRotate}
            title={t("rotate_image")}
          >
            <RotateCw size={14} />
          </button>

          <button
            type="button"
            className={`pin-sticker-btn ${flipped ? "active" : ""}`}
            onClick={() => setFlipped((f) => !f)}
            title={t("flip_image")}
          >
            <FlipHorizontal size={14} />
          </button>

          <button
            type="button"
            className={`pin-sticker-btn ${isAlwaysOnTop ? "active" : ""}`}
            onClick={handleToggleAlwaysOnTop}
            title={t("always_on_top")}
          >
            {isAlwaysOnTop ? <Pin size={14} /> : <PinOff size={14} />}
          </button>

          <button
            type="button"
            className="pin-sticker-btn"
            onClick={handleReset}
            title={t("reset_view")}
          >
            <RotateCcw size={14} />
          </button>

          <div className="pin-sticker-divider" />

          <button
            type="button"
            className="pin-sticker-btn danger"
            onClick={handleClose}
            title={t("close_sticker")}
          >
            <X size={14} />
          </button>
        </div>

        {/* Sticker Image Viewport */}
        <div className="pin-sticker-image-wrapper">
          {imageError ? (
            <div style={{ color: "var(--text-primary, #ffffff)", fontSize: 'var(--font-size-sm)', textAlign: "center", padding: "16px" }}>
              {t("sticker_image_load_failed")}
            </div>
          ) : (
            <img
              ref={imgRef}
              src={rawSrc}
              alt="Pinned Sticker"
              className="pin-sticker-img"
              style={{
                transform: `rotate(${rotation}deg) scaleX(${flipped ? -1 : 1})`,
              }}
              draggable={false}
              onLoad={handleImageLoad}
              onError={() => setImageError(true)}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default PinStickerWindow;
