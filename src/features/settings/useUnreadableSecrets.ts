import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Secrets this device can't decrypt (another Windows install or account, a
 * missing key file). Their settings section gets a "!" instead of a dialog;
 * opening it clears the unreadable value, which can't be recovered here, so
 * the field starts empty for the user to fill in again.
 * Keys from database::SENSITIVE_KEYS; values are section ids in SettingsPanel.
 */
const SECTION_OF_SECRET: Record<string, string> = {
    mqtt_username: "sync",
    mqtt_password: "sync",
    cloud_sync_webdav_password: "cloud_sync",
    cloud_sync_api_key: "cloud_sync",
    e2e_key: "cloud_sync",
    ai_profiles: "ai",
};

export const useUnreadableSecrets = () => {
    const [keys, setKeys] = useState<string[]>([]);

    useEffect(() => {
        invoke<string[]>("get_undecryptable_settings")
            .then(setKeys)
            .catch((error) => console.error("Failed to list unreadable settings", error));
    }, []);

    const flaggedSections = useMemo(
        () => new Set(keys.map((key) => SECTION_OF_SECRET[key]).filter(Boolean)),
        [keys]
    );

    const acknowledge = useCallback((section: string) => {
        const sectionKeys = keys.filter((key) => SECTION_OF_SECRET[key] === section);
        if (sectionKeys.length === 0) return;
        setKeys((current) => current.filter((key) => !sectionKeys.includes(key)));
        invoke("clear_undecryptable_settings", { keys: sectionKeys })
            .catch((error) => console.error("Failed to clear unreadable settings", error));
    }, [keys]);

    return { flaggedSections, acknowledge };
};
