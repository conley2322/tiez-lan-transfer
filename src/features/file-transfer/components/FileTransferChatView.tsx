import { Fragment, useState, useEffect, useRef } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { Check, Copy, Folder, Wifi, Search } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { QRCodeCanvas } from "qrcode.react";

import type {
    FileTransferChatViewProps,
    FileTransferContextMenu,
    FileTransferMessage,
    FileTransferDevice,
    NetworkInterfaceInfo,
    ExpandedTransferItem,
    FileTransferFilterType,
} from "../types";
import { withNativeDialog } from "../../../shared/lib/focus";
import {
    getTransferFileKind,
    formatTransferSpeed,
    MultiFileSendMode,
    MULTI_FILE_SEND_MODE_KEY,
    isLocalTransferPath,
    formatMessageTime,
    getMessageDayKey,
    formatMessageDay,
    messagesBelongToSameGroup,
    resolveDesktopDownloadUrl,
    mediaFallbackKey,
    resolveShareableDownloadUrl,
    buildPairingUrl,
    copyPrivateText,
} from "../utils/transferHelpers";
import {
    PendingTransferItem,
    resolveDropPaths,
    getDroppedTransferItems,
    isExternalFileDrag,
} from "../utils/dragDropHelpers";

import { TransferHeader } from "./TransferHeader";
import { TransferFilterBar } from "./TransferFilterBar";
import { TransferBatchBubble } from "./TransferBatchBubble";
import { TransferMessageItem } from "./TransferMessageItem";
import { TransferPeerAvatar } from "./TransferPeerAvatar";
import { TransferComposer } from "./TransferComposer";
import { TransferContextMenu } from "./TransferContextMenu";
import ConfirmDialog from "../../../shared/components/ConfirmDialog";
import { formatError } from "../../../shared/lib/errorMessage";
import { unfinishedPending } from "../utils/pendingMatch";

const FileTransferChatView = ({
    t,
    localIp,
    actualPort,
    accessToken,
    lanKey,
}: FileTransferChatViewProps) => {
    const [interfaces, setInterfaces] = useState<NetworkInterfaceInfo[]>([]);
    const [selectedIp, setSelectedIp] = useState<string>(localIp);
    const [copiedMessageId, setCopiedMessageId] = useState<number | null>(null);
    const [copiedLink, setCopiedLink] = useState(false);
    const activeIp = selectedIp || localIp;
    const connectionUrl = buildPairingUrl(activeIp, actualPort, accessToken, lanKey);
    const [messages, setMessages] = useState<FileTransferMessage[]>([]);
    const [mediaFallbackSources, setMediaFallbackSources] = useState<Record<string, string>>({});
    const [input, setInput] = useState("");
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const chatBoxRef = useRef<HTMLDivElement>(null);
    const [isUserScrolling, setIsUserScrolling] = useState(false);
    const prevMessagesLengthRef = useRef(0);
    const [contextMenu, setContextMenu] = useState<FileTransferContextMenu | null>(null);
    const [onlineDevices, setOnlineDevices] = useState<FileTransferDevice[]>([]);
    const [isDragging, setIsDragging] = useState(false);
    const isDraggingRef = useRef(false);

    const setDragging = (value: boolean) => {
        isDraggingRef.current = value;
        setIsDragging(value);
    };

    const [showQrCode, setShowQrCode] = useState(false);
    const [isChoosingFiles, setIsChoosingFiles] = useState(false);
    const [multiFileSendMode, setMultiFileSendMode] = useState<MultiFileSendMode>(() => {
        try {
            return localStorage.getItem(MULTI_FILE_SEND_MODE_KEY) === "separate" ? "separate" : "package";
        } catch {
            return "package";
        }
    });
    const [expandedBatches, setExpandedBatches] = useState<Set<string>>(() => new Set());
    const [savingBatchId, setSavingBatchId] = useState<string | null>(null);
    const [showClearConfirm, setShowClearConfirm] = useState(false);
    const [isClearing, setIsClearing] = useState(false);
    const [filterType, setFilterType] = useState<FileTransferFilterType>("all");
    const [searchQuery, setSearchQuery] = useState<string>("");
    const [showSearchBar, setShowSearchBar] = useState<boolean>(false);
    const searchInputRef = useRef<HTMLInputElement>(null);

    const hasOnlineDevices = onlineDevices.length > 0;
    // A device that just joined has scanned the code; put it away.
    const onlineCountRef = useRef(onlineDevices.length);
    useEffect(() => {
        if (onlineDevices.length > onlineCountRef.current) setShowQrCode(false);
        onlineCountRef.current = onlineDevices.length;
    }, [onlineDevices.length]);
    const primaryDeviceName =
        typeof onlineDevices[0]?.name === "string" && onlineDevices[0].name.trim()
            ? onlineDevices[0].name.trim()
            : t("ft_connected_device");
    const conversationTitle =
        onlineDevices.length > 1
            ? t("ft_conversation_multi").replace("{name}", primaryDeviceName).replace("{n}", String(onlineDevices.length))
            : primaryDeviceName;

    // Load available network interfaces
    useEffect(() => {
        let mounted = true;
        const loadInterfaces = async () => {
            try {
                const list = await invoke<NetworkInterfaceInfo[]>("get_available_network_interfaces");
                if (mounted && Array.isArray(list)) {
                    setInterfaces(list);
                }
            } catch (e) {
                console.warn("Failed to load network interfaces:", e);
            }
        };
        loadInterfaces();
        return () => {
            mounted = false;
        };
    }, []);

    useEffect(() => {
        if (localIp) {
            setSelectedIp(localIp);
        }
    }, [localIp]);

    const handleSwitchIp = async (newIp: string) => {
        setSelectedIp(newIp);
        try {
            await invoke("set_display_ip", { ip: newIp });
        } catch (e) {
            console.error("Failed to switch IP:", e);
        }
    };

    const handleCopyLink = async (url: string) => {
        try {
            // Token and key in one link: keep it out of clipboard history.
            await copyPrivateText(url);
            setCopiedLink(true);
            setTimeout(() => setCopiedLink(false), 2000);
        } catch (e) {
            console.warn("Copy link failed:", e);
        }
    };

    const handleOpenFolder = async () => {
        try {
            await invoke("open_file_transfer_folder");
        } catch (e) {
            console.error("Failed to open transfer folder", e);
            void emit("toast", t("ft_open_folder_failed"));
        }
    };

    const handleClearHistory = async () => {
        setIsClearing(true);
        try {
            await invoke("clear_chat_history");
            setMessages([]);
            setShowClearConfirm(false);
            void emit("toast", t("ft_history_cleared"));
        } catch (e) {
            console.error("Failed to clear chat history", e);
            void emit("toast", t("ft_clear_failed"));
        } finally {
            setIsClearing(false);
        }
    };

    const uploadDroppedFile = async (
        file: File,
        batch?: { id: string; name: string; index: number; total: number; totalSize: number },
        onProgress?: (progress: number, speed: string, transferredBytes: number) => void
    ) => {
        const port = Number(actualPort);
        if (!Number.isFinite(port) || port <= 0) {
            throw new Error("File transfer server is not running");
        }

        const CHUNK_SIZE = 1024 * 512;
        const totalChunks = Math.max(1, Math.ceil(file.size / CHUNK_SIZE));
        const uploadId = `desktop_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
        const startTime = performance.now();
        let lastLoggedTime = startTime;
        let lastLoggedBytes = 0;

        for (let i = 0; i < totalChunks; i++) {
            const start = i * CHUNK_SIZE;
            const end = Math.min(file.size, start + CHUNK_SIZE);
            const chunk = file.slice(start, end);
            const formData = new FormData();
            formData.append("file", chunk, file.name);
            formData.append(
                "metadata",
                JSON.stringify({
                    upload_id: uploadId,
                    chunk_index: i,
                    total_chunks: totalChunks,
                    file_name: file.name,
                    sender_id: "pc",
                    sender_name: "电脑",
                    total_size: file.size,
                    content_type: file.type || "application/octet-stream",
                    batch_id: batch?.id,
                    batch_name: batch?.name,
                    batch_index: batch?.index,
                    batch_total: batch?.total,
                    batch_size: batch?.totalSize,
                })
            );

            let lastErr: Error | null = null;
            let success = false;
            for (let attempt = 0; attempt < 3; attempt++) {
                try {
                    const response = await fetch(`http://127.0.0.1:${port}/share-chunk`, {
                        method: "POST",
                        headers: { Authorization: `Bearer ${accessToken}` },
                        body: formData,
                    });

                    if (response.ok) {
                        success = true;
                        break;
                    }
                    const errorText = await response.text().catch(() => "");
                    lastErr = new Error(errorText || `Share chunk upload failed: ${response.status}`);
                    if (
                        response.status >= 400 &&
                        response.status < 500 &&
                        response.status !== 408 &&
                        response.status !== 429
                    ) {
                        break;
                    }
                } catch (err) {
                    lastErr = err instanceof Error ? err : new Error(String(err));
                }
                if (attempt < 2) {
                    await new Promise((r) => setTimeout(r, 400 * (attempt + 1)));
                }
            }

            if (!success) {
                throw lastErr || new Error(t("ft_upload_chunk_failed"));
            }

            const now = performance.now();
            const timeElapsed = (now - lastLoggedTime) / 1000;
            const bytesSinceLast = end - lastLoggedBytes;
            let currentSpeed = "";
            if (timeElapsed >= 0.2 || i === totalChunks - 1) {
                const bps = bytesSinceLast / Math.max(timeElapsed, 0.05);
                currentSpeed = formatTransferSpeed(bps);
                lastLoggedTime = now;
                lastLoggedBytes = end;
            }
            const percent = Math.min(100, Math.round(((i + 1) / totalChunks) * 100));
            onProgress?.(percent, currentSpeed, end);
        }
    };

    const retrySendFile = async (message: FileTransferMessage) => {
        if (!message._rawFile) return;
        const file = message._rawFile;
        setMessages((prev) =>
            prev.map((m) => {
                if (m.id === message.id) {
                    return {
                        ...m,
                        _preparing: true,
                        _error: undefined,
                        _progress: 0,
                        _speed: "",
                        _transferredBytes: 0,
                    };
                }
                return m;
            })
        );

        try {
            await uploadDroppedFile(
                file,
                message.batch_id
                    ? {
                          id: message.batch_id,
                          name: message.batch_name || "文件包",
                          index: message.batch_index || 0,
                          total: message.batch_total || 1,
                          totalSize: message.batch_size || file.size,
                      }
                    : undefined,
                (progress, speed, transferredBytes) => {
                    setMessages((prev) =>
                        prev.map((m) => {
                            if (m.id === message.id) {
                                return {
                                    ...m,
                                    _progress: progress,
                                    _speed: speed || m._speed,
                                    _transferredBytes: transferredBytes,
                                };
                            }
                            return m;
                        })
                    );
                }
            );
            setTimeout(fetchMessages, 300);
        } catch (err) {
            const reason = err instanceof Error ? formatError(err, t) : t("ft_upload_failed");
            setMessages((prev) =>
                prev.map((m) => {
                    if (m.id === message.id) {
                        return {
                            ...m,
                            _preparing: false,
                            _error: reason,
                        };
                    }
                    return m;
                })
            );
        }
    };

    const queueFilesForSending = async (
        rawItems: PendingTransferItem[],
        sendMode: MultiFileSendMode = multiFileSendMode,
        customBatchName?: string
    ) => {
        const items = rawItems.filter((item) => {
            if (item.path && item.path.trim().length > 0) return true;
            return !!item.file;
        });
        if (items.length === 0) return;
        const folderBatchName = items.find((i) => i.folderName)?.folderName
            ? `${items.find((i) => i.folderName)?.folderName} (${items.length} 个文件)`
            : undefined;
        const shouldPackage =
            items.length > 1 &&
            (sendMode === "package" || !!customBatchName || !!folderBatchName);
        const batchId = shouldPackage
            ? `desktop_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
            : undefined;
        const batchName = shouldPackage
            ? customBatchName || folderBatchName || `${items.length} 个文件`
            : undefined;
        const batchSize = items.reduce((total, item) => total + (item.file?.size || 0), 0);

        const tempMessages: FileTransferMessage[] = items.map((item, index) => ({
            id: Date.now() + Math.random(),
            direction: "out",
            msg_type: "file",
            content: "Preparing...",
            timestamp: Date.now(),
            _fileName: item.name,
            _preparing: true,
            _progress: 0,
            _speed: "",
            _transferredBytes: 0,
            _rawFile: item.file,
            batch_id: batchId,
            batch_name: batchName,
            batch_index: batchId ? index : undefined,
            batch_total: batchId ? items.length : undefined,
            batch_size: batchId ? batchSize : undefined,
            file_size: item.file?.size,
        }));

        setMessages((prev) => [...prev, ...tempMessages]);
        setTimeout(() => messagesEndRef.current?.scrollIntoView({ behavior: "smooth" }), 100);

        let results: PromiseSettledResult<unknown>[];
        if (shouldPackage && items.every((item) => !!item.path)) {
            const result = await Promise.allSettled([
                invoke("send_files_to_client", {
                    filePaths: items.map((item) => item.path!),
                    batchName,
                }),
            ]);
            results = items.map(() => result[0]);
        } else {
            results = await Promise.allSettled(
                items.map((item, index) => {
                    if (item.path) {
                        return invoke("send_file_to_client", { filePath: item.path });
                    }
                    if (item.file) {
                        const tempMsgId = tempMessages[index].id;
                        return uploadDroppedFile(
                            item.file,
                            batchId
                                ? {
                                      id: batchId,
                                      name: batchName!,
                                      index,
                                      total: items.length,
                                      totalSize: batchSize,
                                  }
                                : undefined,
                            (progress, speed, transferredBytes) => {
                                setMessages((prev) =>
                                    prev.map((msg) => {
                                        if (msg.id === tempMsgId) {
                                            return {
                                                ...msg,
                                                _progress: progress,
                                                _speed: speed || msg._speed,
                                                _transferredBytes: transferredBytes,
                                            };
                                        }
                                        return msg;
                                    })
                                );
                            }
                        );
                    }
                    return Promise.reject(new Error("No valid file source"));
                })
            );
        }

        results.forEach((result, index) => {
            if (result.status === "rejected") {
                const targetId = tempMessages[index].id;
                const reason =
                    result.reason instanceof Error ? result.reason.message : t("ft_send_failed");
                setMessages((prev) =>
                    prev.map((msg) => {
                        if (msg.id === targetId) {
                            return {
                                ...msg,
                                _preparing: false,
                                _error: reason,
                            };
                        }
                        return msg;
                    })
                );
                console.error(`Failed to send file: ${items[index].name}`, result.reason);
            }
        });

        // Sent: the history has the real message now, drop its bubble.
        const sentIds = new Set(
            tempMessages.filter((_, index) => results[index].status === "fulfilled").map((m) => m.id)
        );
        if (sentIds.size > 0) {
            setMessages((prev) => prev.filter((msg) => !sentIds.has(msg.id)));
            setTimeout(fetchMessages, 300);
        }
    };

    const selectMultiFileSendMode = (mode: MultiFileSendMode) => {
        setMultiFileSendMode(mode);
        try {
            localStorage.setItem(MULTI_FILE_SEND_MODE_KEY, mode);
        } catch {
            // Optional preference
        }
        void chooseFilesToSend(mode);
    };

    // The drop listeners are set up once: they call the current version, which
    // knows the send mode (packaged or one by one) chosen since.
    const queueFilesForSendingRef = useRef(queueFilesForSending);
    useEffect(() => {
        queueFilesForSendingRef.current = queueFilesForSending;
    });

    const chooseFilesToSend = async (sendMode: MultiFileSendMode = multiFileSendMode) => {
        if (isChoosingFiles) return;
        setIsChoosingFiles(true);

        try {
            const selected = await withNativeDialog(
                () =>
                    open({
                        multiple: true,
                        title: t("ft_choose_files_title"),
                    }),
                "file-transfer:choose-files"
            );

            if (!selected) return;
            const paths = Array.isArray(selected) ? selected : [selected];
            await queueFilesForSending(
                paths.map((path) => ({
                    name: path.split(/[/\\]/).pop() || t("type_file"),
                    path,
                })),
                sendMode
            );
        } catch (error) {
            console.error("Failed to choose files", error);
            await emit("toast", t("ft_file_picker_failed"));
        } finally {
            setIsChoosingFiles(false);
        }
    };

    useEffect(() => {
        const activeKeys = new Set(messages.map(mediaFallbackKey));
        setMediaFallbackSources((current) => {
            const entries = Object.entries(current);
            if (entries.every(([key]) => activeKeys.has(key))) return current;
            return Object.fromEntries(entries.filter(([key]) => activeKeys.has(key)));
        });
    }, [messages]);

    const getMessageMediaSrc = (message: FileTransferMessage) => {
        const fallback = mediaFallbackSources[mediaFallbackKey(message)];
        if (fallback) return fallback;
        if (message.content.startsWith("data:")) return message.content;
        if (message.content.startsWith("/download/")) {
            return resolveDesktopDownloadUrl(message.content);
        }
        if (isLocalTransferPath(message.file_path)) return convertFileSrc(message.file_path!);
        if (isLocalTransferPath(message.content)) return convertFileSrc(message.content);
        return message.content;
    };

    const handleMediaPreviewError = async (
        message: FileTransferMessage,
        target: HTMLImageElement | HTMLVideoElement
    ) => {
        if (target.dataset.fallbackAttempted === "true") return;
        target.dataset.fallbackAttempted = "true";

        const localPath = [message.file_path, message.content].find((value) =>
            isLocalTransferPath(value)
        );
        let fallbackPath: string | undefined;

        if (localPath) {
            try {
                fallbackPath = await invoke<string>("get_download_url", { filePath: localPath });
            } catch (error) {
                console.error("Failed to create media preview URL", error);
            }
        }
        fallbackPath ||= message.content.startsWith("/download/")
            ? message.content
            : undefined;
        if (!fallbackPath) return;

        const fallbackSource = resolveDesktopDownloadUrl(fallbackPath);
        if (!/^http:\/\/127\.0\.0\.1:/i.test(fallbackSource)) return;
        setMediaFallbackSources((current) => ({
            ...current,
            [mediaFallbackKey(message)]: fallbackSource,
        }));
    };

    const openTransferContent = async ({
        filePath,
        content,
        type,
    }: {
        filePath?: string;
        content?: string;
        type?: string;
    }) => {
        const localTarget = [filePath, content].find((value) => isLocalTransferPath(value));
        const rawTarget = localTarget || filePath || content;
        if (!rawTarget) {
            await emit("toast", t("ft_nothing_to_open"));
            return;
        }

        const isRemoteTarget = !localTarget && /^(?:https?:|\/download\/)/i.test(rawTarget);
        const target = isRemoteTarget ? resolveShareableDownloadUrl(rawTarget) : rawTarget;

        try {
            await invoke("open_content", {
                id: 0,
                content: target,
                contentType: isRemoteTarget ? "url" : type || "file",
            });
        } catch (error) {
            console.error("Failed to open transferred content", error);
            await emit("toast", t("ft_open_content_failed"));
        }
    };

    const openContextMenu = (
        event: ReactMouseEvent<HTMLElement>,
        menu: Omit<FileTransferContextMenu, "x" | "y">
    ) => {
        event.preventDefault();
        event.stopPropagation();
        setContextMenu({
            ...menu,
            x: event.clientX,
            y: event.clientY,
        });
    };

    const handleCopyText = async (messageId: number, text: string) => {
        await navigator.clipboard.writeText(text);
        setCopiedMessageId(messageId);
        setTimeout(() => {
            setCopiedMessageId((prev) => (prev === messageId ? null : prev));
        }, 1500);
    };

    const toggleBatchExpanded = (batchId: string) => {
        setExpandedBatches((current) => {
            const next = new Set(current);
            if (next.has(batchId)) next.delete(batchId);
            else next.add(batchId);
            return next;
        });
    };

    const saveBatchAs = async (batchMessages: FileTransferMessage[]) => {
        const batchId = batchMessages[0]?.batch_id;
        if (!batchId || savingBatchId) return;
        const sourcePaths = batchMessages
            .map((message) => message.file_path)
            .filter((path): path is string => !!path && isLocalTransferPath(path));
        if (sourcePaths.length !== batchMessages.length) {
            await emit("toast", t("ft_batch_not_ready"));
            return;
        }

        const targetDir = await withNativeDialog(
            () => open({ directory: true, multiple: false, title: t("ft_choose_batch_folder") }),
            "file-transfer:save-batch"
        );
        if (!targetDir || Array.isArray(targetDir)) return;

        setSavingBatchId(batchId);
        try {
            const savedPath = await invoke<string>("save_file_batch", {
                sourcePaths,
                targetDir,
                batchName: batchMessages[0].batch_name || `${batchMessages.length} 个文件`,
            });
            await emit("toast", t("ft_batch_saved").replace("{path}", savedPath));
        } catch (error) {
            console.error("Failed to save file package", error);
            await emit("toast", t("ft_batch_save_failed"));
        } finally {
            setSavingBatchId(null);
        }
    };

    const handleProcessDroppedDataTransfer = async (dt: DataTransfer | null) => {
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
            try {
                const expanded = await invoke<ExpandedTransferItem[]>("expand_transfer_paths", { paths });
                if (expanded.length === 0) {
                    await emit("toast", t("ft_folder_empty"));
                    return;
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
                    await queueFilesForSending(items, "package", `${folderName} (${items.length} 个文件)`);
                }
                if (looseItems.length > 0) {
                    await queueFilesForSending(looseItems);
                }
                return;
            } catch (e) {
                console.warn("expand_transfer_paths failed in HTML drop, falling back", e);
            }
        }

        const items = getDroppedTransferItems(dt);
        if (items.length > 0) {
            void queueFilesForSending(items);
            return;
        }

        void emit("toast", t("ft_drop_unrecognized"));
    };
    const handleProcessDroppedDataTransferRef = useRef(handleProcessDroppedDataTransfer);
    useEffect(() => {
        handleProcessDroppedDataTransferRef.current = handleProcessDroppedDataTransfer;
    });

    const send = async () => {
        if (!input.trim()) return;
        try {
            await invoke("send_chat_message", { msgType: "text", content: input });
            setInput("");
            fetchMessages();
        } catch (e) {
            console.error("Failed to send message", e);
        }
    };

    const fetchMessages = async () => {
        try {
            const msgs = await invoke<FileTransferMessage[]>("get_chat_history");
            setMessages((prev) => {
                const pending = prev.filter((m) => m._preparing || m._error);
                if (pending.length === 0) return msgs;
                return [...msgs, ...unfinishedPending(pending, msgs)];
            });
        } catch (e) {
            console.warn("fetchMessages failed", e);
        }
    };

    // Global drag & drop, device listeners, chunk progress
    useEffect(() => {
        const appWindow = getCurrentWindow();
        fetchMessages();

        const handleDragDrop = async (event: { payload: unknown }) => {
            setDragging(false);
            const paths = resolveDropPaths(event.payload);

            if (paths && paths.length > 0) {
                try {
                    const expanded = await invoke<ExpandedTransferItem[]>("expand_transfer_paths", { paths });
                    if (expanded.length === 0) {
                        await emit("toast", t("ft_folder_empty"));
                        return;
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
                        await queueFilesForSendingRef.current(items, "package", `${folderName} (${items.length} 个文件)`);
                    }
                    if (looseItems.length > 0) {
                        await queueFilesForSendingRef.current(looseItems);
                    }
                } catch (err) {
                    console.error("Failed to expand dropped paths", err);
                    void queueFilesForSendingRef.current(
                        paths.map((path) => ({
                            name: path.split(/[/\\]/).pop() || "File",
                            path,
                        }))
                    );
                }
            }
        };

        const handleDragEnter = () => setDragging(true);
        const handleDragLeave = () => setDragging(false);

        const unlistenV2Drop = appWindow.listen("tauri://drag-drop", handleDragDrop);
        const unlistenV2Enter = appWindow.listen("tauri://drag-enter", handleDragEnter);
        const unlistenV2Leave = appWindow.listen("tauri://drag-leave", handleDragLeave);

        const unlistenDevices = listen<FileTransferDevice[]>("online-devices-updated", (event) => {
            setOnlineDevices(event.payload || []);
        });
        // Phones that connected before this window opened.
        invoke<FileTransferDevice[]>("get_online_devices")
            .then((devices) => setOnlineDevices(devices || []))
            .catch(console.error);

        const unlistenNewMsg = listen<FileTransferMessage>("new-chat-message", () => {
            fetchMessages();
        });

        const unlistenClearHistory = listen("chat-history-cleared", () => {
            setMessages([]);
        });

        interface ChunkProgressPayload {
            upload_id: string;
            file_name: string;
            chunk_index: number;
            total_chunks: number;
            total_size: number;
            sender_id: string;
            sender_name: string;
            batch_id?: string;
            batch_name?: string;
            batch_index?: number;
            batch_total?: number;
            batch_size?: number;
        }

        const unlistenChunkProgress = listen<ChunkProgressPayload>(
            "file-transfer-chunk-progress",
            (event) => {
                const p = event.payload;
                if (!p || !p.upload_id) return;
                const percent = Math.min(
                    100,
                    Math.round(((p.chunk_index + 1) / p.total_chunks) * 100)
                );
                const transferred = Math.min(p.total_size, (p.chunk_index + 1) * 1024 * 512);

                setMessages((prev) => {
                    const existingIndex = prev.findIndex((m) => m._uploadId === p.upload_id);
                    if (existingIndex >= 0) {
                        const existing = prev[existingIndex];
                        const updated = [...prev];
                        updated[existingIndex] = {
                            ...existing,
                            _progress: percent,
                            _transferredBytes: transferred,
                        };
                        return updated;
                    } else {
                        const kind = getTransferFileKind(p.file_name);
                        const msgType =
                            kind === "image" ? "image" : kind === "video" ? "video" : "file";
                        const newPending: FileTransferMessage = {
                            id: Date.now() + Math.random(),
                            direction: "in",
                            msg_type: msgType,
                            content: t("ft_receiving_placeholder"),
                            timestamp: Date.now(),
                            sender_id: p.sender_id,
                            sender_name: p.sender_name || t("ft_phone_device"),
                            _fileName: p.file_name,
                            _uploadId: p.upload_id,
                            _preparing: true,
                            _progress: percent,
                            _transferredBytes: transferred,
                            file_size: p.total_size,
                            batch_id: p.batch_id,
                            batch_name: p.batch_name,
                            batch_index: p.batch_index,
                            batch_total: p.batch_total,
                            batch_size: p.batch_size,
                        };
                        return [...prev, newPending];
                    }
                });
            }
        );

        return () => {
            unlistenV2Drop.then((f) => f());
            unlistenV2Enter.then((f) => f());
            unlistenV2Leave.then((f) => f());
            unlistenDevices.then((f) => f());
            unlistenNewMsg.then((f) => f());
            unlistenClearHistory.then((f) => f());
            unlistenChunkProgress.then((f) => f());
        };
    }, []);

    // Disable navigation gestures inside file transfer view
    useEffect(() => {
        invoke("set_navigation_enabled", { enabled: false }).catch(console.error);
        return () => {
            invoke("set_navigation_enabled", { enabled: true }).catch(console.error);
        };
    }, []);

    // Copy hotkey handling
    useEffect(() => {
        const handleCopy = (e: KeyboardEvent) => {
            const isCopy =
                (e.ctrlKey || e.metaKey) &&
                !e.altKey &&
                !e.shiftKey &&
                e.key.toLowerCase() === "c";
            if (!isCopy) return;

            const selection = window.getSelection();
            const text = selection?.toString() || "";
            if (!text) return;

            const root = chatBoxRef.current;
            const anchor = selection?.anchorNode;
            const focus = selection?.focusNode;
            if (!root || !anchor || !focus) return;
            if (!root.contains(anchor) || !root.contains(focus)) return;

            e.preventDefault();
            e.stopPropagation();
            navigator.clipboard.writeText(text).catch(console.error);
        };

        window.addEventListener("keydown", handleCopy, true);
        return () => window.removeEventListener("keydown", handleCopy, true);
    }, []);

    // Search hotkey (Ctrl/Cmd+F)
    useEffect(() => {
        const handleSearchHotkey = (e: KeyboardEvent) => {
            const isFind =
                (e.ctrlKey || e.metaKey) &&
                !e.altKey &&
                !e.shiftKey &&
                e.key.toLowerCase() === "f";
            if (isFind) {
                e.preventDefault();
                e.stopPropagation();
                setShowSearchBar((prev) => {
                    const next = !prev;
                    if (next) setTimeout(() => searchInputRef.current?.focus(), 50);
                    return next;
                });
            } else if (e.key === "Escape" && showSearchBar) {
                setShowSearchBar(false);
            }
        };

        window.addEventListener("keydown", handleSearchHotkey);
        return () => window.removeEventListener("keydown", handleSearchHotkey);
    }, [showSearchBar]);

    // Detect if user scrolled away from bottom
    useEffect(() => {
        const chatBox = chatBoxRef.current;
        if (!chatBox) return;

        const handleScroll = () => {
            const { scrollTop, scrollHeight, clientHeight } = chatBox;
            const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
            setIsUserScrolling(!isAtBottom);
        };

        chatBox.addEventListener("scroll", handleScroll);
        return () => chatBox.removeEventListener("scroll", handleScroll);
    }, []);

    // Smart Scroll to bottom on new messages
    useEffect(() => {
        const hasNewMessages = messages.length > prevMessagesLengthRef.current;
        prevMessagesLengthRef.current = messages.length;

        if (hasNewMessages && !isUserScrolling) {
            messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
        }
    }, [messages, isUserScrolling]);

    // Drag-over, drag-leave, and drop window listeners
    useEffect(() => {
        const handleWindowDragOver = (event: globalThis.DragEvent) => {
            if (!isExternalFileDrag(event.dataTransfer)) return;
            event.preventDefault();
            if (event.dataTransfer) {
                event.dataTransfer.dropEffect = "copy";
            }
            if (!isDraggingRef.current) {
                setDragging(true);
            }
        };

        const handleWindowDragLeave = (event: globalThis.DragEvent) => {
            if (event.relatedTarget === null) {
                setDragging(false);
            }
        };

        const handleWindowDrop = (event: globalThis.DragEvent) => {
            if (!isExternalFileDrag(event.dataTransfer) && !isDraggingRef.current) return;
            event.preventDefault();
            event.stopPropagation();
            setDragging(false);
            void handleProcessDroppedDataTransferRef.current(event.dataTransfer);
        };

        window.addEventListener("dragover", handleWindowDragOver);
        window.addEventListener("dragleave", handleWindowDragLeave);
        window.addEventListener("drop", handleWindowDrop);

        return () => {
            window.removeEventListener("dragover", handleWindowDragOver);
            window.removeEventListener("dragleave", handleWindowDragLeave);
            window.removeEventListener("drop", handleWindowDrop);
        };
    }, []);

    // Safety net for cancelled drag
    useEffect(() => {
        if (!isDragging) return;

        const clearDragging = () => setDragging(false);
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape") clearDragging();
        };

        window.addEventListener("keydown", handleKeyDown);
        window.addEventListener("dragend", clearDragging);
        window.addEventListener("blur", clearDragging);

        return () => {
            window.removeEventListener("keydown", handleKeyDown);
            window.removeEventListener("dragend", clearDragging);
            window.removeEventListener("blur", clearDragging);
        };
    }, [isDragging]);

    // Filter and search logic
    const isFiltering = filterType !== "all" || searchQuery.trim().length > 0;
    const normalizedQuery = searchQuery.trim().toLowerCase();

    const matchesFilterCriteria = (m: FileTransferMessage): boolean => {
        if (filterType === "file") {
            const isFile = m.msg_type === "file" || !!m.batch_id;
            if (!isFile) return false;
        } else if (filterType === "media") {
            const isMedia = m.msg_type === "image" || m.msg_type === "video";
            if (!isMedia) return false;
        } else if (filterType === "text") {
            if (m.msg_type !== "text") return false;
        }

        if (normalizedQuery) {
            const contentMatch = m.content?.toLowerCase().includes(normalizedQuery);
            const fileNameMatch = m._fileName?.toLowerCase().includes(normalizedQuery);
            const filePathMatch = m.file_path?.toLowerCase().includes(normalizedQuery);
            const batchNameMatch = m.batch_name?.toLowerCase().includes(normalizedQuery);
            if (!contentMatch && !fileNameMatch && !filePathMatch && !batchNameMatch) {
                return false;
            }
        }

        return true;
    };

    const filteredMessages = messages.filter((m) => {
        if (!isFiltering) return true;
        if (m.batch_id) {
            const batchSiblings = messages.filter((s) => s.batch_id === m.batch_id);
            const batchNameMatches = m.batch_name?.toLowerCase().includes(normalizedQuery);
            if (batchNameMatches) {
                if (filterType === "text") return false;
                if (filterType === "media")
                    return m.msg_type === "image" || m.msg_type === "video";
                return true;
            }
            const anySiblingMatches = batchSiblings.some((s) => matchesFilterCriteria(s));
            if (anySiblingMatches) {
                return matchesFilterCriteria(m);
            }
            return false;
        }
        return matchesFilterCriteria(m);
    });

    return (
        <div
            className="wt-chat-view"
            style={{ position: "relative" }}
            onContextMenu={(e) => {
                if (!(e.target as HTMLElement).closest(".wt-context-menu")) {
                    e.preventDefault();
                }
            }}
            onDragOver={(e) => {
                if (!isExternalFileDrag(e.dataTransfer)) return;
                e.preventDefault();
                if (e.dataTransfer) {
                    e.dataTransfer.dropEffect = "copy";
                }
                if (!isDraggingRef.current) setDragging(true);
            }}
            onDragLeave={(e) => {
                if (e.currentTarget.contains(e.relatedTarget as Node)) return;
                setDragging(false);
            }}
            onDrop={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setDragging(false);
                void handleProcessDroppedDataTransfer(e.dataTransfer);
            }}
        >
            {/* Drag Overlay */}
            <AnimatePresence>
                {isDragging && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        className="wt-drag-overlay"
                        onContextMenu={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            setDragging(false);
                        }}
                        onClick={() => setDragging(false)}
                    >
                        <div className="wt-drag-drop-zone">
                            <Folder size={64} strokeWidth={1.5} />
                            <div className="wt-drag-drop-text">{t("ft_drop_to_send")}</div>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>

            {/* Header */}
            {localIp && actualPort && accessToken && (
                <TransferHeader
                    t={t}
                    hasOnlineDevices={hasOnlineDevices}
                    conversationTitle={conversationTitle}
                    onlineDevicesCount={onlineDevices.length}
                    connectionUrl={connectionUrl}
                    activeIp={activeIp}
                    actualPort={actualPort}
                    selectedIp={selectedIp}
                    interfaces={interfaces}
                    onSwitchIp={handleSwitchIp}
                    copiedLink={copiedLink}
                    onCopyLink={handleCopyLink}
                    showSearchBar={showSearchBar}
                    isFiltering={isFiltering}
                    onToggleSearch={() => {
                        setShowSearchBar((prev) => {
                            const next = !prev;
                            if (next) setTimeout(() => searchInputRef.current?.focus(), 50);
                            return next;
                        });
                    }}
                    onOpenFolder={handleOpenFolder}
                    hasMessages={messages.length > 0}
                    onClearHistory={() => setShowClearConfirm(true)}
                    showQrCode={showQrCode}
                    onToggleQrCode={() => setShowQrCode((prev) => !prev)}
                />
            )}

            {/* Search & Filter Bar */}
            <TransferFilterBar
                t={t}
                showSearchBar={showSearchBar}
                filterType={filterType}
                onSelectFilterType={setFilterType}
                searchQuery={searchQuery}
                onSearchQueryChange={setSearchQuery}
                searchInputRef={searchInputRef}
                isFiltering={isFiltering}
                filteredCount={filteredMessages.length}
            />

            <ConfirmDialog
                open={showClearConfirm}
                title={t("ft_clear_history")}
                message={t("ft_clear_confirm_message")}
                confirmLabel={isClearing ? t("ft_clearing") : t("ft_clear_confirm")}
                cancelLabel={t("cancel")}
                danger
                busy={isClearing}
                onConfirm={handleClearHistory}
                onClose={() => setShowClearConfirm(false)}
            />

            {/* Chat Box */}
            <div className="wt-chat-box" ref={chatBoxRef}>
                {messages.length === 0 && (
                    <div className="wt-empty-state">
                        {hasOnlineDevices ? (
                            <>
                                <div className="wt-empty-icon" aria-hidden="true">
                                    <Wifi size={22} strokeWidth={1.8} />
                                </div>
                                <div className="wt-empty-title">{t("ft_empty_title")}</div>
                                <div className="wt-empty-copy">
                                    {t("ft_empty_copy")}
                                </div>
                            </>
                        ) : (
                            <div className="wt-pairing-hero">
                                <div className="wt-pairing-qr">
                                    <QRCodeCanvas value={connectionUrl} size={128} />
                                </div>
                                <div className="wt-pairing-title">{t("ft_pair_title")}</div>
                                <div className="wt-pairing-tip">
                                    {t("ft_pair_tip")}
                                </div>
                                <div className="wt-pairing-address-row">
                                    <span className="wt-pairing-address">{activeIp}:{actualPort}</span>
                                    <button
                                        type="button"
                                        className="wt-pairing-copy-btn"
                                        onClick={() => handleCopyLink(connectionUrl)}
                                        title={t("ft_copy_connect_url")}
                                    >
                                        {copiedLink ? t("ft_copied") : t("ft_copy_address")}
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>
                )}

                {messages.length > 0 && filteredMessages.length === 0 && (
                    <div className="wt-empty-state wt-empty-search">
                        <div className="wt-empty-icon" aria-hidden="true">
                            <Search size={22} strokeWidth={1.8} />
                        </div>
                        <div className="wt-empty-title">{t("ft_no_match_title")}</div>
                        <div className="wt-empty-copy">
                            {t("ft_no_match_copy")}
                        </div>
                        <button
                            type="button"
                            className="wt-clear-filter-btn"
                            onClick={() => {
                                setSearchQuery("");
                                setFilterType("all");
                            }}
                        >
                            {t("ft_clear_filters")}
                        </button>
                    </div>
                )}

                {filteredMessages.map((m, index) => {
                    const batchMessages = m.batch_id
                        ? filteredMessages
                              .filter((message) => message.batch_id === m.batch_id)
                              .sort(
                                  (left, right) =>
                                      (left.batch_index ?? 0) - (right.batch_index ?? 0)
                              )
                        : [];
                    const isBatch = !!m.batch_id && (m.batch_total ?? batchMessages.length) > 1;
                    if (
                        isBatch &&
                        filteredMessages.findIndex((message) => message.batch_id === m.batch_id) !== index
                    ) {
                        return null;
                    }
                    const groupedWithPrevious = messagesBelongToSameGroup(
                        filteredMessages[index - 1],
                        m
                    );
                    const groupedWithNext = messagesBelongToSameGroup(
                        m,
                        filteredMessages[index + 1]
                    );
                    const startsNewDay =
                        index === 0 ||
                        getMessageDayKey(filteredMessages[index - 1].timestamp) !==
                            getMessageDayKey(m.timestamp);

                    if (isBatch) {
                        return (
                            <Fragment key={`batch-${m.batch_id}`}>
                                {startsNewDay && m.timestamp > 0 && (
                                    <div className="wt-day-separator" role="separator">
                                        <span>{formatMessageDay(m.timestamp, t)}</span>
                                    </div>
                                )}
                                <div
                                    className={`wt-message ${
                                        m.direction === "out" ? "sent" : "received"
                                    } is-group-start is-group-end`}
                                >
                                    {m.direction !== "out" && <TransferPeerAvatar message={m} />}
                                    <div className="wt-message-stack">
                                        {m.direction !== "out" && m.sender_name && (
                                            <div className="wt-sender-name">{m.sender_name}</div>
                                        )}
                                        <TransferBatchBubble
                                            t={t}
                                            message={m}
                                            batchMessages={batchMessages}
                                            expanded={expandedBatches.has(m.batch_id!)}
                                            onToggleExpand={toggleBatchExpanded}
                                            onOpenContent={openTransferContent}
                                            onSaveBatchAs={saveBatchAs}
                                            isSaving={savingBatchId === m.batch_id}
                                            onContextMenu={openContextMenu}
                                        />
                                        <div className="wt-message-meta">
                                            <span>{formatMessageTime(m.timestamp)}</span>
                                            {m.direction === "out" && (
                                                <span className="wt-delivery-mark">✓</span>
                                            )}
                                        </div>
                                    </div>
                                </div>
                            </Fragment>
                        );
                    }

                    return (
                        <Fragment key={m.id}>
                            {startsNewDay && m.timestamp > 0 && (
                                <div className="wt-day-separator" role="separator">
                                    <span>{formatMessageDay(m.timestamp, t)}</span>
                                </div>
                            )}
                            <div
                                className={`wt-message ${
                                    m.direction === "out" ? "sent" : "received"
                                } ${
                                    groupedWithPrevious
                                        ? "is-grouped-with-previous"
                                        : "is-group-start"
                                } ${
                                    groupedWithNext ? "is-grouped-with-next" : "is-group-end"
                                }`}
                            >
                                {/* Incoming only: the avatar sits on a group's last message. */}
                                {m.direction !== "out" &&
                                    (groupedWithNext ? (
                                        <div className="wt-avatar-spacer" aria-hidden="true" />
                                    ) : (
                                        <TransferPeerAvatar message={m} />
                                    ))}
                                <div className="wt-message-stack">
                                    {m.direction !== "out" && m.sender_name && !groupedWithPrevious && (
                                        <div className="wt-sender-name">{m.sender_name}</div>
                                    )}
                                    <div className="wt-bubble-anchor">
                                        <TransferMessageItem
                                            t={t}
                                            message={m}
                                            getMessageMediaSrc={getMessageMediaSrc}
                                            onMediaPreviewError={handleMediaPreviewError}
                                            onOpenContent={openTransferContent}
                                            onRetrySend={retrySendFile}
                                            onContextMenu={openContextMenu}
                                        />
                                        {/* Beside the bubble, not over its text. */}
                                        {m.msg_type === "text" && (
                                            <button
                                                type="button"
                                                className={`wt-copy-btn${copiedMessageId === m.id ? " is-copied" : ""}`}
                                                title={copiedMessageId === m.id ? t("ft_copied") : t("ft_copy_content")}
                                                aria-label={copiedMessageId === m.id ? t("ft_copied") : t("ft_copy_content")}
                                                onClick={() => void handleCopyText(m.id, m.content)}
                                            >
                                                {copiedMessageId === m.id ? <Check size={14} /> : <Copy size={14} />}
                                            </button>
                                        )}
                                    </div>
                                    {/* One time per group, under its last message. */}
                                    {!groupedWithNext && (
                                        <div className="wt-message-meta">
                                            <span>{formatMessageTime(m.timestamp)}</span>
                                            {m.direction === "out" && (
                                                <span className="wt-delivery-mark">✓</span>
                                            )}
                                        </div>
                                    )}
                                </div>
                            </div>
                        </Fragment>
                    );
                })}
                <div ref={messagesEndRef} />
            </div>

            {/* Composer */}
            <TransferComposer
                input={input}
                setInput={setInput}
                onSend={send}
                multiFileSendMode={multiFileSendMode}
                onSelectMultiFileSendMode={selectMultiFileSendMode}
                chooseFilesToSend={chooseFilesToSend}
                isChoosingFiles={isChoosingFiles}
                t={t}
            />

            {/* Context Menu */}
            <TransferContextMenu
                contextMenu={contextMenu}
                onClose={() => setContextMenu(null)}
                expandedBatches={expandedBatches}
                toggleBatchExpanded={toggleBatchExpanded}
                savingBatchId={savingBatchId}
                saveBatchAs={saveBatchAs}
                openTransferContent={openTransferContent}
                t={t}
                localIp={activeIp}
                actualPort={actualPort}
            />
        </div>
    );
};

export default FileTransferChatView;
