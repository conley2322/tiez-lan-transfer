import { useState, useEffect, useCallback, useRef } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getReleaseBuildInfo, type ReleaseChannel } from "../lib/releaseBuild";
import { isTauriRuntime } from "../lib/tauriRuntime";

export type UpdateStatus = "idle" | "checking" | "downloading" | "ready" | "error";
export type UpdateCheckResultStatus = "available" | "up-to-date" | "error" | "portable";

export interface UpdateCheckResult {
  requestId?: string;
  status: UpdateCheckResultStatus;
  version?: string;
  message?: string;
}

interface ManualUpdateRequest {
  requestId?: string;
}

interface CheckOutcome extends UpdateCheckResult {
  update?: Update;
}

const CHECK_REQUEST_EVENT = "check-update-manually";
export const CHECK_RESULT_EVENT = "update-check-result";

// Stable builds also look at the beta channel. With "receive beta updates"
// off, a newer beta is only shown as a line in Settings (no dialog); clicking
// it opens the usual dialog, marked as a beta.
export const RECEIVE_BETA_SETTING = "app.receive_beta_updates";
export const BETA_AVAILABLE_EVENT = "beta-update-available";
const OPEN_BETA_UPDATE_EVENT = "open-beta-update";
let availableBetaVersion: string | null = null;
export const getAvailableBetaVersion = () => availableBetaVersion;
export const requestBetaUpdateDialog = () => emit(OPEN_BETA_UPDATE_EVENT);

/** SemVer pre-releases (1.0.0-beta.5) are beta builds. */
export const isBetaVersion = (version: string) => version.includes("-");

const checkBetaChannel = async (includeStable: boolean): Promise<Update | null> => {
  const metadata = await invoke<ConstructorParameters<typeof Update>[0] | null>(
    "check_beta_update",
    { includeStable }
  );
  return metadata ? new Update(metadata) : null;
};

const receivesBetaUpdates = async () => {
  const settings = await invoke<Record<string, string>>("get_settings").catch(() => ({} as Record<string, string>));
  return settings[RECEIVE_BETA_SETTING] === "true";
};

export const useAutoUpdate = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [version, setVersion] = useState("");
  const [notes, setNotes] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(0);
  const [updateObj, setUpdateObj] = useState<Update | null>(null);
  const [important, setImportant] = useState(false);
  const [releaseChannel, setReleaseChannel] = useState<ReleaseChannel>("stable");
  const checkPromiseRef = useRef<Promise<CheckOutcome> | null>(null);
  const betaUpdateRef = useRef<Update | null>(null);
  const downloadedBytesRef = useRef(0);
  const totalBytesRef = useRef<number | null>(null);

  const performCheck = useCallback(async (): Promise<CheckOutcome> => {
    if (!isTauriRuntime()) return { status: "up-to-date" };

    const buildInfo = await getReleaseBuildInfo();
    setReleaseChannel(buildInfo.channel);
    if (buildInfo.portable) {
      setStatus("idle");
      return { status: "portable" };
    }

    try {
      setStatus("checking");
      const stableBuild = buildInfo.channel === "stable";
      const optedIntoBeta = stableBuild && await receivesBetaUpdates();
      // Opted in: the beta channel returns the newer of beta and stable.
      const update = optedIntoBeta
        ? await checkBetaChannel(true)
        : await check({
            headers: { "Cache-Control": "no-cache" },
            timeout: 10000,
          });

      if (!update) {
        if (stableBuild && !optedIntoBeta) {
          // A failed beta lookup must not turn "up to date" into an error.
          const beta = await checkBetaChannel(false).catch(() => null);
          betaUpdateRef.current?.close().catch(() => {});
          betaUpdateRef.current = beta;
          availableBetaVersion = beta?.version ?? null;
          await emit(BETA_AVAILABLE_EVENT, availableBetaVersion);
        }
        setStatus("idle");
        return { status: "up-to-date" };
      }

      console.log(`[Update] New ${buildInfo.channel} version detected: ${update.version}`);
      setUpdateObj(update);
      setVersion(update.version);
      setNotes(update.body || "");
      setImportant(update.rawJson?.forceUpdate === true || update.rawJson?.important === true);
      setStatus("idle");
      setIsOpen(true);
      return { status: "available", version: update.version, update };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error("[Update] Failed to check for updates:", error);
      setStatus("error");
      return { status: "error", message };
    }
  }, []);

  const checkUpdate = useCallback(async (requestId?: string) => {
    if (!isTauriRuntime()) return;

    if (!checkPromiseRef.current) {
      checkPromiseRef.current = performCheck().finally(() => {
        checkPromiseRef.current = null;
      });
    }

    const outcome = await checkPromiseRef.current;
    if (requestId) {
      const { update: _update, ...result } = outcome;
      await emit<UpdateCheckResult>(CHECK_RESULT_EVENT, { ...result, requestId });
    }
  }, [performCheck]);

  const startUpdate = useCallback(async () => {
    if (!updateObj || status === "downloading") return;

    try {
      setStatus("downloading");
      setDownloadProgress(0);
      downloadedBytesRef.current = 0;
      totalBytesRef.current = null;

      await updateObj.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            totalBytesRef.current = event.data.contentLength ?? null;
            setDownloadProgress(totalBytesRef.current ? 0 : null);
            console.log(`[Update] Download started (${event.data.contentLength ?? "unknown"} bytes)`);
            break;
          case "Progress": {
            downloadedBytesRef.current += event.data.chunkLength;
            const total = totalBytesRef.current;
            if (total && total > 0) {
              setDownloadProgress(Math.min((downloadedBytesRef.current / total) * 100, 99));
            } else {
              setDownloadProgress(null);
            }
            break;
          }
          case "Finished":
            console.log("[Update] Download finished");
            setDownloadProgress(100);
            break;
        }
      });

      setStatus("ready");
      setDownloadProgress(100);
    } catch (error) {
      console.error("[Update] Failed to download or install update:", error);
      setStatus("error");
    }
  }, [status, updateObj]);

  const applyUpdate = useCallback(async () => {
    try {
      await relaunch();
    } catch (error) {
      console.error("[Update] Failed to relaunch:", error);
      setStatus("error");
      setIsOpen(true);
    }
  }, []);

  // From the line in Settings: show the beta in the usual dialog. The update
  // handle is released when the dialog closes, so look it up again if needed.
  const openBetaUpdate = useCallback(async () => {
    if (isOpen || status === "downloading") return;
    const beta = betaUpdateRef.current ?? await checkBetaChannel(false).catch(() => null);
    betaUpdateRef.current = null;
    if (!beta) {
      availableBetaVersion = null;
      await emit(BETA_AVAILABLE_EVENT, null);
      return;
    }
    setUpdateObj(beta);
    setVersion(beta.version);
    setNotes(beta.body || "");
    setImportant(false);
    setStatus("idle");
    setIsOpen(true);
  }, [isOpen, status]);

  const closeUpdateDialog = useCallback(() => {
    if (status === "downloading") return;
    setIsOpen(false);
    setStatus("idle");
    setDownloadProgress(0);
    setImportant(false);
    if (updateObj) {
      updateObj.close().catch((error) => {
        console.warn("[Update] Failed to release updater resource:", error);
      });
      setUpdateObj(null);
    }
  }, [status, updateObj]);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    getReleaseBuildInfo().then((buildInfo) => {
      setReleaseChannel(buildInfo.channel);
    });

    const timer = setTimeout(() => {
      checkUpdate();
    }, 5000);

    let unlisten: (() => void) | undefined;
    listen<ManualUpdateRequest>(CHECK_REQUEST_EVENT, (event) => {
      checkUpdate(event.payload?.requestId);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      clearTimeout(timer);
      unlisten?.();
    };
  }, [checkUpdate]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let unlisten: (() => void) | undefined;
    listen(OPEN_BETA_UPDATE_EVENT, () => {
      void openBetaUpdate();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [openBetaUpdate]);

  return {
    isOpen,
    status,
    version,
    notes,
    important,
    releaseChannel,
    // The badge in the dialog describes the offered version, not this build:
    // a beta build being offered 1.0.0 is offered a stable release.
    offeredChannel: (isBetaVersion(version) ? "beta" : "stable") as ReleaseChannel,
    downloadProgress,
    onManualUpdate: checkUpdate,
    onStartDownload: startUpdate,
    onApplyUpdate: applyUpdate,
    onClose: closeUpdateDialog,
  };
};
