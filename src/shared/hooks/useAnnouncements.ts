import { useState, useEffect, useCallback, useRef } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../lib/tauriRuntime";
import type { Announcement } from "../types/announcement";

const DISMISSED_ANNOUNCEMENTS_KEY = "app.dismissed_announcements";
const LEGACY_DISMISSED_ANNOUNCEMENTS_KEY = "dismissed_announcements";

function parseDismissedAnnouncements(value?: string | null): string[] {
  if (!value) return [];

  try {
    const parsed: unknown = JSON.parse(value);
    return Array.isArray(parsed)
      ? parsed.filter((id): id is string => typeof id === "string")
      : [];
  } catch {
    return [];
  }
}

async function getDismissedAnnouncements(): Promise<string[]> {
  const legacyDismissed = parseDismissedAnnouncements(
    localStorage.getItem(LEGACY_DISMISSED_ANNOUNCEMENTS_KEY)
  );

  try {
    const settings = await invoke<Record<string, string>>("get_settings");
    const persistedDismissed = parseDismissedAnnouncements(
      settings[DISMISSED_ANNOUNCEMENTS_KEY]
    );
    const dismissed = [...new Set([...persistedDismissed, ...legacyDismissed])];

    // Migrate the old WebView-only value into the portable data directory.
    if (legacyDismissed.some((id) => !persistedDismissed.includes(id))) {
      await invoke("save_setting", {
        key: DISMISSED_ANNOUNCEMENTS_KEY,
        value: JSON.stringify(dismissed),
      });
    }

    return dismissed;
  } catch (error) {
    console.warn("[Announcements] Failed to load dismissed announcements:", error);
    return legacyDismissed;
  }
}

export function buildPingUrl(pingUrl: string, version: string, pingId: string | null): string {
  const url = new URL(pingUrl);
  if (pingId) url.searchParams.set("id", pingId);
  url.searchParams.set("v", version);
  return url.toString();
}

export function useAnnouncements() {
  const [announcements, setAnnouncements] = useState<Announcement[]>([]);
  const [loading, setLoading] = useState(true);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchAnnouncements = useCallback(async () => {
    if (!isTauriRuntime()) {
      setLoading(false);
      return;
    }

    try {
      const pingUrl =
        import.meta.env.VITE_TIEZ_ANNOUNCEMENT_PING_URL ||
        import.meta.env.VITE_PING_URL ||
        `${import.meta.env.VITE_API_BASE_URL || "https://tiez.name666.top"}/api/v1/ping`;
      
      // One id per device, shared with the startup ping. A device keeps the
      // id this window made before, so an update doesn't count it as new; the
      // data folder keeps a copy in case WebView storage is cleared.
      const windowId = localStorage.getItem("device_id");
      const pingId = await invoke<string>("get_ping_id", { windowId }).catch(() => windowId);
      if (pingId) localStorage.setItem("device_id", pingId);

      const version = await getVersion();
      const url = buildPingUrl(pingUrl, version, pingId);

      const response = await fetch(url, {
        signal: AbortSignal.timeout(5000),
      });

      if (!response.ok) throw new Error("Ping failed");

      const data = await response.json();
      const fetchedBroadcasts = data.broadcasts || [];

      const dismissed = await getDismissedAnnouncements();
      
      const validAnnouncements = fetchedBroadcasts.filter(
        (a: Announcement) => !dismissed.includes(a.id)
      );

      setAnnouncements(validAnnouncements);
    } catch (error) {
      console.warn("[Announcements] Fetch failed:", error);
      setAnnouncements([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAnnouncements();

    const CHECK_INTERVAL = 6 * 60 * 60 * 1000;
    timerRef.current = setInterval(fetchAnnouncements, CHECK_INTERVAL);

    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [fetchAnnouncements]);

  const dismissAnnouncement = async (id: string) => {
    setAnnouncements((prev) => prev.filter((a) => a.id !== id));
    const dismissed = await getDismissedAnnouncements();
    if (!dismissed.includes(id)) {
      dismissed.push(id);
      const value = JSON.stringify(dismissed);

      // Keep this fallback for existing installs and transient database errors.
      localStorage.setItem(LEGACY_DISMISSED_ANNOUNCEMENTS_KEY, value);
      try {
        await invoke("save_setting", {
          key: DISMISSED_ANNOUNCEMENTS_KEY,
          value,
        });
      } catch (error) {
        console.warn("[Announcements] Failed to persist dismissed announcement:", error);
      }
    }
  };

  return { announcements, loading, dismissAnnouncement, refetch: fetchAnnouncements };
}
