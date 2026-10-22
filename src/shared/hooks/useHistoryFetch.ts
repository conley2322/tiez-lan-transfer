import { useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Dispatch, SetStateAction } from "react";
import type { ClipboardEntry } from "../types";
import type { ClipboardSortMode } from "../../features/app/types";
import { isTauriRuntime } from "../lib/tauriRuntime";
import { parseSearchQuery } from "../lib/searchQuery";

interface UseHistoryFetchOptions {
  debouncedSearch: string;
  typeFilter: string | null;
  sortMode: ClipboardSortMode;
  persistentLimitEnabled: boolean;
  persistentLimit: number;
  pageSize: number;
  currentOffset: number;
  historyLength: number;
  setHistory: Dispatch<SetStateAction<ClipboardEntry[]>>;
  setCurrentOffset: Dispatch<SetStateAction<number>>;
  setHasMore: Dispatch<SetStateAction<boolean>>;
  isLoadingMore: boolean;
  hasMore: boolean;
  setIsLoadingMore: Dispatch<SetStateAction<boolean>>;
}

export const useHistoryFetch = ({
  debouncedSearch,
  typeFilter,
  sortMode,
  persistentLimitEnabled,
  persistentLimit,
  pageSize,
  currentOffset,
  setHistory,
  setCurrentOffset,
  setHasMore,
  isLoadingMore,
  hasMore,
  setIsLoadingMore
}: UseHistoryFetchOptions) => {
  const loadingRef = useRef(false);
  const fetchSeqRef = useRef(0);
  const lastRequestedOffsetRef = useRef<number | null>(null);
  const currentOffsetRef = useRef(currentOffset);

  useEffect(() => {
    currentOffsetRef.current = currentOffset;
  }, [currentOffset]);

  const fetchHistory = useCallback(
    // keepLoaded: reload as many entries as are loaded now (after a change),
    // instead of the first page, so the list doesn't jump back to the top.
    async (reset = false, keepLoaded = false) => {
      if (!isTauriRuntime()) {
        if (reset) {
          setHistory([]);
          setCurrentOffset(0);
        }
        setHasMore(false);
        return;
      }

      const seq = ++fetchSeqRef.current;
      try {
        if (reset) {
          lastRequestedOffsetRef.current = null;
        }

        const baseOffset = reset
          ? 0
          : currentOffsetRef.current;

        let data: ClipboardEntry[] = [];

        const hasSearch = debouncedSearch && debouncedSearch.trim().length > 0;

        if (hasSearch) {
          const parsed = parseSearchQuery(debouncedSearch);

          try {
            const rawResults = await invoke<ClipboardEntry[]>("search_clipboard_history", {
              searchTerm: parsed.term,
              limit: 200,
              tagOnly: parsed.tagOnly,
              noteOnly: parsed.noteOnly
            });
            data = rawResults.map(item => ({ ...item, _serverSearchMatched: parsed.raw }));
          } catch (e) {
            console.error("Search failed, falling back", e);
            data = [];
          }

          if (seq !== fetchSeqRef.current) return;
          // Search results are not paginated; always replace list and stop infinite loading.
          setHistory(data);
          setCurrentOffset(data.length);
          setHasMore(false);
        } else {
          const loadCount = reset && keepLoaded
            ? Math.max(pageSize, currentOffsetRef.current)
            : pageSize;
          const requestedLimit = loadCount + 1;
          const rawData = await invoke<ClipboardEntry[]>("get_clipboard_history", {
            limit: requestedLimit,
            offset: baseOffset,
            contentType: typeFilter || undefined,
            sortMode
          });

          if (seq !== fetchSeqRef.current) return;

          const hasMoreNow = rawData.length > loadCount;
          const data = hasMoreNow ? rawData.slice(0, loadCount) : rawData;

          // Track total retrieved items across both session and DB items
          const itemsCount = data.length;

          if (reset) {
            setHistory(data);
            setCurrentOffset(itemsCount);
            setHasMore(hasMoreNow);
          } else {
            let nextItems: ClipboardEntry[] = [];
            setHistory((prev) => {
              const existingIds = new Set(prev.map((item) => item.id));
              nextItems = data.filter((item) => !existingIds.has(item.id) || item.id === 0);

              if (nextItems.length === 0) return prev;
              return [...prev, ...nextItems];
            });

            setCurrentOffset(prev => prev + itemsCount);
            // If we didn't add any NEW items but the backend says there are more,
            // it means the items we got were already in our list (maybe shifted due to sorting).
            // We should keep hasMore true so the user can try to load further.
            setHasMore(hasMoreNow);
          }
        }
      } catch (err) {
        console.error("无法获取历史记录", err);
        if (seq === fetchSeqRef.current) {
          lastRequestedOffsetRef.current = null;
        }
      }
    },
    [
      debouncedSearch,
      typeFilter,
      sortMode,
      pageSize,
      persistentLimit,
      persistentLimitEnabled,
      setCurrentOffset,
      setHasMore,
      setHistory
    ]
  );

  const loadMoreHistory = useCallback(async () => {
    if (loadingRef.current || isLoadingMore || !hasMore) return;
    if (debouncedSearch && debouncedSearch.trim().length > 0) return;

    const effectiveOffset = currentOffsetRef.current;
    if (lastRequestedOffsetRef.current === effectiveOffset) return;
    lastRequestedOffsetRef.current = effectiveOffset;

    loadingRef.current = true;
    setIsLoadingMore(true);
    try {
      await fetchHistory(false);
    } finally {
      loadingRef.current = false;
      setIsLoadingMore(false);
    }
  }, [debouncedSearch, fetchHistory, hasMore, isLoadingMore, setIsLoadingMore]);

  return { fetchHistory, loadMoreHistory };
};
