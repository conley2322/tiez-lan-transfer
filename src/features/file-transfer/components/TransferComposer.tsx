import { useState, useRef, useEffect } from "react";
import type { KeyboardEvent, ClipboardEvent } from "react";
import {
    Plus,
    Maximize2,
    Minimize2,
    Send,
    LoaderCircle,
    ChevronDown,
    FileArchive,
    File as FileIcon,
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { MultiFileSendMode } from "../utils/transferHelpers";

interface TransferComposerProps {
    input: string;
    setInput: (value: string | ((prev: string) => string)) => void;
    onSend: () => void;
    multiFileSendMode: MultiFileSendMode;
    onSelectMultiFileSendMode: (mode: MultiFileSendMode) => void;
    chooseFilesToSend: (mode?: MultiFileSendMode) => Promise<void>;
    isChoosingFiles: boolean;
    t: (key: string) => string;
}

export const TransferComposer = ({
    input,
    setInput,
    onSend,
    multiFileSendMode,
    onSelectMultiFileSendMode,
    chooseFilesToSend,
    isChoosingFiles,
    t,
}: TransferComposerProps) => {
    const composerMinHeight = 22;
    const [showFullScreen, setShowFullScreen] = useState(false);
    const [showExpandBtn, setShowExpandBtn] = useState(false);
    const [showFileSendOptions, setShowFileSendOptions] = useState(false);
    const [isMultiLine, setIsMultiLine] = useState(false);
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const fileSendOptionsRef = useRef<HTMLDivElement>(null);

    // Auto-adjust textarea height
    useEffect(() => {
        const textarea = textareaRef.current;
        if (!textarea) return;

        // When input is empty, strictly enforce single line
        if (!input) {
            textarea.style.height = `${composerMinHeight}px`;
            setIsMultiLine(false);
            setShowExpandBtn(false);
            return;
        }

        // Reset to minimum height to read scrollHeight accurately
        textarea.style.height = `${composerMinHeight}px`;
        const scrollHeight = textarea.scrollHeight;

        const multiLine = input.includes("\n") || scrollHeight > 35;
        setIsMultiLine(multiLine);

        if (multiLine) {
            const targetHeight = Math.min(Math.max(scrollHeight, composerMinHeight), 120);
            textarea.style.height = `${targetHeight}px`;
            setShowExpandBtn(scrollHeight > 120);
        } else {
            textarea.style.height = `${composerMinHeight}px`;
            setShowExpandBtn(false);
        }
    }, [input]);

    // Click outside to dismiss file options dropdown
    useEffect(() => {
        if (!showFileSendOptions) return;
        const closeOptions = (event: globalThis.MouseEvent) => {
            if (!fileSendOptionsRef.current?.contains(event.target as Node)) {
                setShowFileSendOptions(false);
            }
        };
        window.addEventListener("pointerdown", closeOptions);
        return () => window.removeEventListener("pointerdown", closeOptions);
    }, [showFileSendOptions]);

    const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
        // Enter that picks an input method candidate isn't "send". WebKit ends
        // the composition before that keydown, so check its keyCode (229) too.
        if (e.nativeEvent.isComposing || e.keyCode === 229) return;
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    };

    const handlePaste = async (e: ClipboardEvent<HTMLTextAreaElement>) => {
        if (e.clipboardData.files.length > 0) {
            const files = Array.from(e.clipboardData.files);
            const imageFiles = files.filter((f) => f.type.startsWith("image/"));

            if (imageFiles.length > 0) {
                e.preventDefault();

                for (const file of imageFiles) {
                    const reader = new FileReader();
                    reader.onload = async (ev) => {
                        const base64 = ev.target?.result as string;
                        if (base64) {
                            try {
                                const savedPath = await invoke<string>("save_temp_image", { base64Data: base64 });
                                await invoke("send_file_to_client", { filePath: savedPath });
                            } catch (err) {
                                console.error("Failed to paste image", err);
                            }
                        }
                    };
                    reader.readAsDataURL(file);
                }
            }
        }
    };

    const handleSend = () => {
        if (!input.trim()) return;
        onSend();
        setShowFullScreen(false);
        setIsMultiLine(false);
        if (textareaRef.current) {
            textareaRef.current.style.height = `${composerMinHeight}px`;
        }
    };

    return (
        <>
            <div className="wt-footer">
                <div className="wt-composer">
                    <div className="wt-file-send-control" ref={fileSendOptionsRef}>
                        <button
                            type="button"
                            className="wt-btn-icon wt-file-send-main"
                            title={
                                isChoosingFiles
                                    ? t("ft_choosing_files")
                                    : multiFileSendMode === "package"
                                        ? t("ft_send_files_package")
                                        : t("ft_send_files_separate")
                            }
                            aria-label={isChoosingFiles ? t("ft_choosing_files") : t("ft_send_files")}
                            disabled={isChoosingFiles}
                            onClick={() => void chooseFilesToSend()}
                        >
                            {isChoosingFiles ? (
                                <LoaderCircle size={18} className="wt-spin" />
                            ) : (
                                <Plus size={18} />
                            )}
                        </button>
                        <button
                            type="button"
                            className="wt-file-send-toggle"
                            aria-label={t("ft_send_mode_choose")}
                            aria-expanded={showFileSendOptions}
                            disabled={isChoosingFiles}
                            onClick={() => setShowFileSendOptions((value) => !value)}
                        >
                            <ChevronDown size={12} />
                        </button>
                        {showFileSendOptions && (
                            <div className="wt-file-send-options" role="menu">
                                <button
                                    type="button"
                                    className={multiFileSendMode === "package" ? "is-active" : ""}
                                    role="menuitemradio"
                                    aria-checked={multiFileSendMode === "package"}
                                    onClick={() => {
                                        onSelectMultiFileSendMode("package");
                                        setShowFileSendOptions(false);
                                    }}
                                >
                                    <FileArchive size={17} />
                                    <span>
                                        <strong>{t("ft_send_as_package")}</strong>
                                        <small>{t("ft_send_as_package_hint")}</small>
                                    </span>
                                    {multiFileSendMode === "package" && (
                                        <span className="wt-option-check">✓</span>
                                    )}
                                </button>
                                <button
                                    type="button"
                                    className={multiFileSendMode === "separate" ? "is-active" : ""}
                                    role="menuitemradio"
                                    aria-checked={multiFileSendMode === "separate"}
                                    onClick={() => {
                                        onSelectMultiFileSendMode("separate");
                                        setShowFileSendOptions(false);
                                    }}
                                >
                                    <FileIcon size={17} />
                                    <span>
                                        <strong>{t("ft_send_separately")}</strong>
                                        <small>{t("ft_send_separately_hint")}</small>
                                    </span>
                                    {multiFileSendMode === "separate" && (
                                        <span className="wt-option-check">✓</span>
                                    )}
                                </button>
                            </div>
                        )}
                    </div>

                    <div className={`wt-input-wrap ${isMultiLine ? "is-multiline" : ""}`}>
                        <textarea
                            ref={textareaRef}
                            className="wt-input"
                            value={input}
                            onChange={(e) => setInput(e.target.value)}
                            onKeyDown={handleKeyDown}
                            onPaste={handlePaste}
                            placeholder={t("type_message")}
                            rows={1}
                        />
                    </div>

                    {showExpandBtn && (
                        <button
                            type="button"
                            className="wt-btn-icon"
                            onClick={() => setShowFullScreen(true)}
                            title={t("ft_expand_editor")}
                            aria-label={t("ft_expand_editor")}
                        >
                            <Maximize2 size={16} />
                        </button>
                    )}

                    <button
                        type="button"
                        onClick={handleSend}
                        className="wt-btn send"
                        disabled={!input.trim()}
                        title={t("ft_send_message")}
                        aria-label={t("ft_send_message")}
                    >
                        <Send size={18} />
                    </button>
                </div>
            </div>

            <AnimatePresence>
                {showFullScreen && (
                    <motion.div
                        initial={{ opacity: 0, scale: 0.95 }}
                        animate={{ opacity: 1, scale: 1 }}
                        exit={{ opacity: 0, scale: 0.95 }}
                        className="wt-fullscreen-editor"
                    >
                        <div className="wt-fullscreen-header">
                            <div className="wt-fullscreen-title">{t("ft_edit_message")}</div>
                            <button
                                onClick={() => setShowFullScreen(false)}
                                className="wt-overlay-icon-btn"
                                title={t("ft_collapse_editor")}
                                aria-label={t("ft_collapse_editor")}
                            >
                                <Minimize2 size={16} />
                            </button>
                        </div>

                        <textarea
                            value={input}
                            onChange={(e) => setInput(e.target.value)}
                            placeholder={t("ft_message_placeholder")}
                            className="wt-fullscreen-textarea"
                            onPaste={handlePaste}
                        />

                        <div className="wt-fullscreen-footer">
                            <button
                                onClick={() => setShowFullScreen(false)}
                                className="wt-btn"
                            >
                                {t("cancel")}
                            </button>
                            <button
                                onClick={handleSend}
                                className="wt-btn send"
                            >
                                {t("ft_send")}
                            </button>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </>
    );
};
