import { useState, useEffect } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
    Image as ImageIcon,
    Video,
    FileText,
    LoaderCircle,
    FileArchive,
    Music,
    FileCode,
    Cpu,
    FileSpreadsheet,
    Presentation,
    File as FileIcon,
} from "lucide-react";
import type { FileTransferMessage } from "../types";
import { getFileIcon as getSystemFileIcon, peekFileIcon } from "../../../shared/lib/fileIcon";

export type TransferFileKind =
    | 'archive'
    | 'audio'
    | 'executable'
    | 'pdf'
    | 'document'
    | 'spreadsheet'
    | 'presentation'
    | 'code'
    | 'image'
    | 'video'
    | 'file';

export const getTransferFileKind = (fileName: string): TransferFileKind => {
    const extension = fileName.split('.').pop()?.toLowerCase() || '';
    if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'].includes(extension)) return 'archive';
    if (['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg'].includes(extension)) return 'audio';
    if (['exe', 'msi', 'bat', 'sh', 'app', 'dmg', 'pkg'].includes(extension)) return 'executable';
    if (extension === 'pdf') return 'pdf';
    if (['doc', 'docx', 'txt', 'rtf', 'md'].includes(extension)) return 'document';
    if (['xls', 'xlsx', 'csv', 'numbers'].includes(extension)) return 'spreadsheet';
    if (['ppt', 'pptx', 'key'].includes(extension)) return 'presentation';
    if (['js', 'ts', 'tsx', 'jsx', 'py', 'rs', 'c', 'cpp', 'go', 'java', 'html', 'css', 'json', 'yaml', 'yml', 'toml', 'sql'].includes(extension)) return 'code';
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'heic', 'avif'].includes(extension)) return 'image';
    if (['mp4', 'mov', 'avi', 'mkv', 'webm', 'm4v'].includes(extension)) return 'video';
    return 'file';
};

export const TransferFileTypeIcon = ({
    filePath,
    fileName,
    preparing
}: {
    filePath?: string;
    fileName: string;
    preparing?: boolean;
}) => {
    const [systemIcon, setSystemIcon] = useState<string | null>(() => peekFileIcon(filePath) ?? null);
    const kind = getTransferFileKind(fileName);

    useEffect(() => {
        let cancelled = false;
        if (!filePath) {
            setSystemIcon(null);
            return () => { cancelled = true; };
        }

        const cached = peekFileIcon(filePath);
        if (cached !== undefined) {
            setSystemIcon(cached ?? null);
            return () => { cancelled = true; };
        }

        setSystemIcon(null);
        getSystemFileIcon(filePath).then(icon => {
            if (!cancelled) setSystemIcon(icon);
        });
        return () => { cancelled = true; };
    }, [filePath]);

    if (preparing) return <LoaderCircle size={22} className="wt-spin" />;
    if (systemIcon) {
        return <img src={systemIcon} alt="" className="wt-file-system-icon" loading="lazy" />;
    }

    const iconByKind: Record<TransferFileKind, ReactNode> = {
        archive: <FileArchive size={23} />,
        audio: <Music size={23} />,
        executable: <Cpu size={23} />,
        pdf: <FileText size={23} />,
        document: <FileText size={23} />,
        spreadsheet: <FileSpreadsheet size={23} />,
        presentation: <Presentation size={23} />,
        code: <FileCode size={23} />,
        image: <ImageIcon size={23} />,
        video: <Video size={23} />,
        file: <FileIcon size={23} />
    };

    return <span className={`wt-file-fallback-icon is-${kind}`}>{iconByKind[kind]}</span>;
};

export const isLocalTransferPath = (path?: string) => {
    if (!path || /^(?:https?:|data:|blob:|asset:|file:|\/download\/)/i.test(path)) {
        return false;
    }
    return /^[a-z]:[\\/]/i.test(path) || /^\\\\/.test(path) || path.startsWith('/');
};

export const decodeFileName = (value: string) => {
    try {
        return decodeURIComponent(value.replace(/\+/g, ' '));
    } catch {
        return value;
    }
};

export const getTransferFileName = (message: FileTransferMessage, t: (key: string) => string) => {
    if (message._fileName?.trim()) return message._fileName.trim();

    for (const value of [message.content, message.file_path]) {
        if (!value) continue;
        const queryName = value.match(/(?:^|[?&])name=([^&]+)/i)?.[1];
        if (queryName) return decodeFileName(queryName);

        const withoutQuery = value.split(/[?#]/)[0];
        const baseName = withoutQuery.split(/[/\\]/).pop();
        if (baseName && !['download', 'file'].includes(baseName.toLowerCase())) {
            return decodeFileName(baseName);
        }
    }

    return t("type_file");
};

export const getTransferFileIconPath = (message: FileTransferMessage) => {
    if (isLocalTransferPath(message.file_path)) return message.file_path;
    if (isLocalTransferPath(message.content)) return message.content;
    return undefined;
};

export const formatTransferBytes = (bytes?: number) => {
    if (!bytes || bytes <= 0) return '';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / Math.pow(1024, unitIndex);
    return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
};

export const formatTransferSpeed = (bytesPerSec: number): string => {
    if (bytesPerSec <= 0 || !Number.isFinite(bytesPerSec)) return '';
    if (bytesPerSec >= 1024 * 1024 * 1024) {
        return `${(bytesPerSec / (1024 * 1024 * 1024)).toFixed(1)} GB/s`;
    }
    if (bytesPerSec >= 1024 * 1024) {
        return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
    }
    if (bytesPerSec >= 1024) {
        return `${(bytesPerSec / 1024).toFixed(0)} KB/s`;
    }
    return `${Math.round(bytesPerSec)} B/s`;
};

export const normalizeTimestamp = (timestamp: number) => (
    timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp
);

export const formatMessageTime = (timestamp: number) => {
    if (!timestamp) return '';
    const normalized = normalizeTimestamp(timestamp);
    const date = new Date(normalized);
    if (Number.isNaN(date.getTime())) return '';
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
};

export const getMessageDayKey = (timestamp: number) => {
    if (!timestamp) return '';
    const date = new Date(normalizeTimestamp(timestamp));
    if (Number.isNaN(date.getTime())) return '';
    return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
};

export const formatMessageDay = (timestamp: number, t: (key: string) => string) => {
    const date = new Date(normalizeTimestamp(timestamp));
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(today.getDate() - 1);

    const key = getMessageDayKey(timestamp);
    if (key === getMessageDayKey(today.getTime())) return t("ft_today");
    if (key === getMessageDayKey(yesterday.getTime())) return t("ft_yesterday");
    return date.toLocaleDateString([], {
        month: 'numeric',
        day: 'numeric',
        ...(date.getFullYear() === today.getFullYear() ? {} : { year: 'numeric' })
    });
};

export const messagesBelongToSameGroup = (
    previous: FileTransferMessage | undefined,
    next: FileTransferMessage | undefined
) => {
    if (!previous || !next || previous.direction !== next.direction) return false;

    const previousSender = previous.sender_id || previous.sender_name || previous.direction;
    const nextSender = next.sender_id || next.sender_name || next.direction;
    if (previousSender !== nextSender) return false;

    const previousTime = normalizeTimestamp(previous.timestamp);
    const nextTime = normalizeTimestamp(next.timestamp);
    return Math.abs(nextTime - previousTime) <= 5 * 60 * 1000;
};

export const normalizeUrl = (raw: string) => {
    if (/^https?:\/\//i.test(raw)) return raw;
    return `http://${raw}`;
};

export const formatHttpHost = (host: string) => (
    host.includes(':') && !host.startsWith('[') ? `[${host}]` : host
);

export const resolveShareableDownloadUrl = (content: string, localIp?: string, actualPort?: string) => {
    if (!content.startsWith('/download/')) return content;
    if (localIp && actualPort) {
        return `http://${formatHttpHost(localIp)}:${actualPort}${content}`;
    }
    return content;
};

export const resolveDesktopDownloadUrl = (content: string, actualPort?: string) => {
    if (!content.startsWith('/download/') || !actualPort) return content;
    return `http://127.0.0.1:${actualPort}${content}`;
};

export const mediaFallbackKey = (message: FileTransferMessage) => (
    `${message.id}:${message.content}:${message.file_path || ''}`
);

export const URL_REGEX = /((https?:\/\/|www\.)[^\s<]+|(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+(?:[a-z]{2,})(?:\/[^\s<]*)?)/gi;

export const splitTrailingPunctuation = (raw: string) => {
    let url = raw;
    let trailing = '';
    while (url.length > 0 && /[)\]}>.,;!?]$/.test(url)) {
        trailing = url.slice(-1) + trailing;
        url = url.slice(0, -1);
    }
    return { url, trailing };
};

export const renderTextWithLinks = (text: string) => {
    const parts: ReactNode[] = [];
    let lastIndex = 0;

    text.replace(URL_REGEX, (match, _group, _proto, offset) => {
        if (match.includes('@')) {
            return match;
        }
        const prevChar = offset > 0 ? text[offset - 1] : '';
        if (prevChar && /[a-z0-9@]/i.test(prevChar)) {
            return match;
        }
        if (offset > lastIndex) {
            parts.push(text.slice(lastIndex, offset));
        }

        const { url, trailing } = splitTrailingPunctuation(match);
        const href = normalizeUrl(url);

        parts.push(
            <a
                key={`link-${offset}`}
                href={href}
                className="wt-link"
                onClick={(e) => {
                    e.preventDefault();
                    if (window.getSelection()?.toString()) return;
                    invoke('open_content', { id: 0, content: href, contentType: 'url' }).catch(console.error);
                }}
            >
                {url}
            </a>
        );

        if (trailing) {
            parts.push(trailing);
        }

        lastIndex = offset + match.length;
        return match;
    });

    if (lastIndex < text.length) {
        parts.push(text.slice(lastIndex));
    }

    return parts.length > 0 ? parts : text;
};

export type MultiFileSendMode = 'package' | 'separate';
export const MULTI_FILE_SEND_MODE_KEY = 'tiez.file-transfer.multi-file-send-mode';

/**
 * The pairing link behind the QR code. The session token rides in the query
 * (the server trades it for a cookie); the encryption key rides in the
 * fragment, which browsers never send over the network
 * (docs/LAN-TRANSFER-ENCRYPTION.md).
 */
export const buildPairingUrl = (ip: string, port: string, token: string, lanKey: string) =>
    `http://${ip}:${port}/?auth=${encodeURIComponent(token)}${lanKey ? `#k=${lanKey}` : ""}`;

/**
 * Copies a link that grants access (pairing link, file download link) marked
 * so clipboard history tools, TieZ included, do not keep it.
 */
export const copyPrivateText = (text: string) =>
    invoke("copy_private_text", { text }).catch(() => navigator.clipboard.writeText(text));
