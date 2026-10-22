import type { MouseEvent as ReactMouseEvent } from "react";
import {
    Image as ImageIcon,
    Video,
    AlertCircle,
    RotateCcw,
} from "lucide-react";
import type { FileTransferMessage, FileTransferContextMenu } from "../types";
import {
    TransferFileTypeIcon,
    getTransferFileName,
    getTransferFileIconPath,
    formatTransferBytes,
    renderTextWithLinks,
} from "../utils/transferHelpers";

export interface TransferMessageItemProps {
    t: (key: string) => string;
    message: FileTransferMessage;
    getMessageMediaSrc: (message: FileTransferMessage) => string;
    onMediaPreviewError: (message: FileTransferMessage, target: HTMLImageElement | HTMLVideoElement) => void;
    onOpenContent: (target: { filePath?: string; content?: string; type?: string }) => void;
    onRetrySend: (message: FileTransferMessage) => void;
    onContextMenu: (event: ReactMouseEvent<HTMLElement>, menu: Omit<FileTransferContextMenu, 'x' | 'y'>) => void;
}

export const TransferMessageItem = ({
    t,
    message: m,
    getMessageMediaSrc,
    onMediaPreviewError,
    onOpenContent,
    onRetrySend,
    onContextMenu,
}: TransferMessageItemProps) => {
    const transferFileName = getTransferFileName(m, t);

    return (
        <div
            className={`wt-bubble wt-bubble-${m.msg_type}`}
            onContextMenu={(event) => {
                if (m._preparing) {
                    event.preventDefault();
                    event.stopPropagation();
                    return;
                }
                const menuType = m.msg_type === 'text' || m.msg_type === 'image' || m.msg_type === 'video'
                    ? m.msg_type
                    : 'file';
                onContextMenu(event, {
                    filePath: menuType === 'text' ? undefined : (m.file_path || m.content),
                    content: m.content,
                    id: m.id,
                    type: menuType
                });
            }}
        >
            {/* Text Message */}
            {m.msg_type === 'text' && (
                <div className="wt-text-wrapper">
                    <div
                        className="wt-text-content"
                        onContextMenu={(e) => onContextMenu(e, {
                            content: m.content,
                            type: 'text'
                        })}
                    >
                        {renderTextWithLinks(m.content)}
                    </div>
                </div>
            )}

            {/* Image Preview */}
            {m.msg_type === 'image' && !m._preparing && !m._error && (
                <>
                    <img
                        src={getMessageMediaSrc(m)}
                        className="wt-img-preview"
                        loading="lazy"
                        draggable={false}
                        style={{ cursor: 'pointer' }}
                        alt="Image"
                        onClick={() => {
                            onOpenContent({
                                filePath: m.file_path,
                                content: m.content,
                                type: 'image'
                            });
                        }}
                        onError={(event) => {
                            onMediaPreviewError(m, event.currentTarget);
                        }}
                        onContextMenu={(e) => onContextMenu(e, {
                            filePath: m.file_path || m.content,
                            content: m.content,
                            id: m.id,
                            type: 'image'
                        })}
                    />
                    <div className="wt-media-footer">
                        <ImageIcon size={12} />
                        <span>{t("type_image")}</span>
                    </div>
                </>
            )}

            {/* Video Preview */}
            {m.msg_type === 'video' && !m._preparing && !m._error && (
                <>
                    <video
                        src={getMessageMediaSrc(m)}
                        className="wt-video-preview"
                        controls
                        onError={(event) => {
                            onMediaPreviewError(m, event.currentTarget);
                        }}
                        onContextMenu={(e) => onContextMenu(e, {
                            filePath: m.file_path || m.content,
                            content: m.content,
                            id: m.id,
                            type: 'video'
                        })}
                    />
                    <div className="wt-media-footer">
                        <Video size={12} />
                        <span>{t("type_video")}</span>
                    </div>
                </>
            )}

            {/* File Card */}
            {((m.msg_type === 'file' || (m.msg_type !== 'text' && m.msg_type !== 'image' && m.msg_type !== 'video')) || m._preparing || m._error) && (
                <>
                    <div
                        className="wt-file-card"
                        style={{ cursor: m.direction === 'in' && !m._preparing && !m._error ? 'pointer' : 'default' }}
                        onClick={() => {
                            if (m.direction === 'in' && !m._preparing && !m._error) {
                                onOpenContent({
                                    filePath: m.file_path,
                                    content: m.content,
                                    type: 'file'
                                });
                            }
                        }}
                        onContextMenu={(e) => {
                            if (m._preparing || m._error) {
                                e.preventDefault();
                                e.stopPropagation();
                                return;
                            }
                            onContextMenu(e, {
                                filePath: m.file_path || m.content,
                                content: m.content,
                                id: m.id,
                                type: 'file'
                            });
                        }}
                    >
                        <div className="wt-file-icon">
                            <TransferFileTypeIcon
                                filePath={getTransferFileIconPath(m)}
                                fileName={transferFileName}
                                preparing={m._preparing}
                            />
                        </div>
                        <div className="wt-file-info">
                            <div className="wt-file-name">{transferFileName}</div>
                            {!m._preparing && !m._error && (
                                <div className="wt-file-status">
                                    {m.direction === 'in' ? t("ft_received_click_open") : t("ft_available_download")}
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Progress Bar */}
                    {m._preparing && (
                        <div className="progress-wrapper">
                            <div className="progress-container">
                                <div
                                    className={`progress-bar${m._progress === undefined ? ' is-indeterminate' : ''}`}
                                    style={{
                                        width: m._progress !== undefined ? `${Math.max(2, m._progress)}%` : '100%'
                                    }}
                                />
                            </div>
                            <div className="status-text">
                                <span className="status-label">
                                    {m.direction === 'in' ? t("ft_receiving") : t("ft_sending")}
                                    {m._speed ? ` · ${m._speed}` : ''}
                                    {m._transferredBytes && m.file_size
                                        ? ` (${formatTransferBytes(m._transferredBytes)} / ${formatTransferBytes(m.file_size)})`
                                        : m.file_size ? ` (${formatTransferBytes(m.file_size)})` : ''}
                                </span>
                                <span className="percent">
                                    {m._progress !== undefined ? `${m._progress}%` : ''}
                                </span>
                            </div>
                        </div>
                    )}

                    {/* Error & Retry */}
                    {m._error && (
                        <div className="wt-transfer-error">
                            <div className="wt-transfer-error-text" title={m._error}>
                                <AlertCircle size={13} />
                                <span>{m._error}</span>
                            </div>
                            {m._rawFile && (
                                <button
                                    type="button"
                                    className="wt-transfer-retry-btn"
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        onRetrySend(m);
                                    }}
                                >
                                    <RotateCcw size={11} />
                                    <span>{t("retry")}</span>
                                </button>
                            )}
                        </div>
                    )}
                </>
            )}
        </div>
    );
};
