import { motion, AnimatePresence } from "framer-motion";
import { Search, X } from "lucide-react";
import type { FileTransferFilterType } from "../types";

export interface TransferFilterBarProps {
    t: (key: string) => string;
    showSearchBar: boolean;
    filterType: FileTransferFilterType;
    onSelectFilterType: (type: FileTransferFilterType) => void;
    searchQuery: string;
    onSearchQueryChange: (query: string) => void;
    searchInputRef: React.RefObject<HTMLInputElement | null>;
    isFiltering: boolean;
    filteredCount: number;
}

export const TransferFilterBar = ({
    t,
    showSearchBar,
    filterType,
    onSelectFilterType,
    searchQuery,
    onSearchQueryChange,
    searchInputRef,
    isFiltering,
    filteredCount,
}: TransferFilterBarProps) => {
    return (
        <AnimatePresence>
            {showSearchBar && (
                <motion.div
                    className="wt-filter-bar"
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.16, ease: "easeOut" }}
                >
                    <div className="wt-filter-bar-inner">
                        <div className="wt-filter-pills" role="tablist">
                            {(
                                [
                                    { id: 'all', label: t("type_all") },
                                    { id: 'file', label: t("type_file") },
                                    { id: 'media', label: t("ft_filter_media") },
                                    { id: 'text', label: t("type_text") },
                                ] as const
                            ).map(pill => (
                                <button
                                    key={pill.id}
                                    type="button"
                                    role="tab"
                                    aria-selected={filterType === pill.id}
                                    className={`wt-filter-pill ${filterType === pill.id ? 'is-active' : ''}`}
                                    onClick={() => onSelectFilterType(pill.id)}
                                >
                                    {pill.label}
                                </button>
                            ))}
                        </div>

                        <div className="wt-filter-search-wrap">
                            <div className="wt-search-input-box">
                                <Search size={13} className="wt-search-icon" />
                                <input
                                    ref={searchInputRef}
                                    type="text"
                                    className="wt-search-input"
                                    placeholder={t("ft_search_placeholder")}
                                    value={searchQuery}
                                    onChange={(e) => onSearchQueryChange(e.target.value)}
                                />
                                {searchQuery && (
                                    <button
                                        type="button"
                                        className="wt-search-clear-btn"
                                        onClick={() => {
                                            onSearchQueryChange('');
                                            searchInputRef.current?.focus();
                                        }}
                                        title={t("ft_clear_search")}
                                        aria-label={t("ft_clear_search")}
                                    >
                                        <X size={12} />
                                    </button>
                                )}
                            </div>

                            {isFiltering && (
                                <div className="wt-filter-badge">
                                    {t("ft_result_count").replace("{n}", String(filteredCount))}
                                </div>
                            )}
                        </div>
                    </div>
                </motion.div>
            )}
        </AnimatePresence>
    );
};
