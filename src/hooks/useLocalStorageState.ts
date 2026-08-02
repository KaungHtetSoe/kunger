import { useEffect, useState } from "react";

/**
 * Like `useState`, but persists to `localStorage` under `key` and
 * initializes from it on mount. Used for view preferences that should
 * survive a restart (Prompt 09C: "Persist view preferences locally") --
 * never for scan data itself, which always comes from the backend cache.
 */
export function useLocalStorageState<T>(key: string, defaultValue: T) {
  const [value, setValue] = useState<T>(() => {
    try {
      const stored = window.localStorage.getItem(key);
      return stored !== null ? (JSON.parse(stored) as T) : defaultValue;
    } catch {
      return defaultValue;
    }
  });

  useEffect(() => {
    try {
      window.localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // Storage can legitimately fail (private browsing, quota) -- a lost
      // view preference is not worth surfacing to the user.
    }
  }, [key, value]);

  return [value, setValue] as const;
}
