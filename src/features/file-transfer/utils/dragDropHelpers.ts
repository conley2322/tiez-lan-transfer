import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import type { ExpandedTransferItem } from "../types";

export type PendingTransferItem = {
    name: string;
    path?: string;
    file?: File;
    folderName?: string;
};

export const resolveDropPaths = (payload: unknown): string[] => {
    if (Array.isArray(payload)) {
        return payload
            .map((p) => (typeof p === "string" ? p.trim() : ""))
            .filter((p) => p.length > 0);
    }
    if (payload && typeof payload === "object" && "paths" in payload) {
        const maybePaths = (payload as { paths?: unknown }).paths;
        if (Array.isArray(maybePaths)) {
            return maybePaths
                .map((p) => (typeof p === "string" ? p.trim() : ""))
                .filter((p) => p.length > 0);
        }
    }
    return [];
};

export const getFilesFromDataTransfer = (dt: DataTransfer | null): File[] => {
    if (!dt) return [];
    const files: File[] = [];
    if (dt.items) {
        for (let i = 0; i < dt.items.length; i++) {
            const item = dt.items[i];
            if (item.kind !== "file") continue;
            const file = item.getAsFile();
            if (file) {
                files.push(file);
            }
        }
        if (files.length > 0) {
            return files;
        }
    }
    if (dt.files && dt.files.length > 0) {
        return Array.from(dt.files);
    }
    return [];
};

export const getDroppedTransferItems = (dt: DataTransfer | null): PendingTransferItem[] => {
    const files = getFilesFromDataTransfer(dt);
    const seen = new Set<string>();
    const items: PendingTransferItem[] = [];

    files.forEach((file) => {
        const maybePath = (file as File & { path?: string }).path?.trim();
        const key = maybePath || `${file.name}:${file.size}:${file.lastModified}`;
        if (seen.has(key)) return;
        seen.add(key);
        items.push({
            name: file.name || maybePath?.split(/[/\\]/).pop() || "File",
            path: maybePath || undefined,
            file: maybePath ? undefined : file
        });
    });

    return items;
};

export const isExternalFileDrag = (dataTransfer: DataTransfer | null | undefined) => {
    if (!dataTransfer) return false;
    return Array.from(dataTransfer.types).includes('Files');
};

export const processDroppedPaths = async (
    paths: string[],
    onQueue: (items: PendingTransferItem[], mode?: 'package' | 'separate', customBatchName?: string) => Promise<void>,
    t: (key: string) => string
): Promise<boolean> => {
    if (!paths || paths.length === 0) return false;
    try {
        const expanded = await invoke<ExpandedTransferItem[]>("expand_transfer_paths", { paths });
        if (expanded.length === 0) {
            await emit("toast", t("ft_folder_empty"));
            return true;
        }

        const folderGroups = new Map<string, PendingTransferItem[]>();
        const looseItems: PendingTransferItem[] = [];

        for (const item of expanded) {
            const transferItem: PendingTransferItem = {
                name: item.name,
                path: item.path,
                folderName: item.folder_name || undefined,
            };
            if (item.folder_name) {
                const list = folderGroups.get(item.folder_name) || [];
                list.push(transferItem);
                folderGroups.set(item.folder_name, list);
            } else {
                looseItems.push(transferItem);
            }
        }

        for (const [folderName, items] of folderGroups.entries()) {
            await onQueue(items, 'package', `${folderName} (${items.length} 个文件)`);
        }
        if (looseItems.length > 0) {
            await onQueue(looseItems);
        }
        return true;
    } catch (err) {
        console.error("Failed to expand dropped paths", err);
        await onQueue(
            paths.map((path) => ({
                name: path.split(/[/\\]/).pop() || "File",
                path
            }))
        );
        return true;
    }
};

export const processDroppedDataTransfer = async (
    dt: DataTransfer | null,
    onQueue: (items: PendingTransferItem[], mode?: 'package' | 'separate', customBatchName?: string) => Promise<void>,
    t: (key: string) => string
) => {
    if (!dt) return;
    const paths: string[] = [];
    if (dt.items) {
        for (let i = 0; i < dt.items.length; i++) {
            const item = dt.items[i];
            const file = item.getAsFile();
            const p = (file as File & { path?: string })?.path;
            if (p) paths.push(p);
        }
    }
    if (paths.length === 0 && dt.files) {
        for (let i = 0; i < dt.files.length; i++) {
            const p = (dt.files[i] as File & { path?: string })?.path;
            if (p) paths.push(p);
        }
    }

    if (paths.length > 0) {
        const handled = await processDroppedPaths(paths, onQueue, t);
        if (handled) return;
    }

    const items = getDroppedTransferItems(dt);
    if (items.length > 0) {
        await onQueue(items);
        return;
    }

    void emit("toast", t("ft_drop_unrecognized"));
};
