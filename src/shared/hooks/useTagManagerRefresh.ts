import { useEffect, useRef } from "react";

interface UseTagManagerRefreshOptions {
  showTagManager: boolean;
  settingsLoaded: boolean;
  persistentLimitEnabled: boolean;
  persistentLimit: number;
  fetchHistory: (reset?: boolean) => void;
}

export const useTagManagerRefresh = ({
  showTagManager,
  settingsLoaded,
  persistentLimitEnabled,
  persistentLimit,
  fetchHistory
}: UseTagManagerRefreshOptions) => {
  // fetchHistory changes identity with search / type / sort, which
  // useSearchFetchTrigger already handles; depending on it here fetched twice.
  const fetchHistoryRef = useRef(fetchHistory);
  fetchHistoryRef.current = fetchHistory;

  useEffect(() => {
    if (!settingsLoaded) return;
    if (!showTagManager) {
      fetchHistoryRef.current(true);
    }
  }, [showTagManager, settingsLoaded, persistentLimitEnabled, persistentLimit]);
};
