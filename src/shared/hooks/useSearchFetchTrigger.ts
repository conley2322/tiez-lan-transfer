import { useEffect, useRef } from "react";

interface UseSearchFetchTriggerOptions {
  debouncedSearch: string;
  isComposing: boolean;
  typeFilter?: string | null;
  sortMode: string;
  fetchHistory: (reset?: boolean) => void;
}

export const useSearchFetchTrigger = ({
  debouncedSearch,
  isComposing,
  typeFilter,
  sortMode,
  fetchHistory
}: UseSearchFetchTriggerOptions) => {
  const fetchHistoryRef = useRef(fetchHistory);
  fetchHistoryRef.current = fetchHistory;

  useEffect(() => {
    if (!isComposing) {
      fetchHistoryRef.current(true);
    }
  }, [debouncedSearch, typeFilter, sortMode, isComposing]);
};
