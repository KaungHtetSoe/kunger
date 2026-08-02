import { useCallback, useMemo, useRef, useState, type ReactNode } from "react";

import {
  NotificationContext,
  type Notification,
  type NotificationLevel,
} from "./notificationContext";

const LEVEL_STYLES: Record<NotificationLevel, string> = {
  info: "border-neutral-700 bg-neutral-900 text-neutral-100",
  success: "border-emerald-700 bg-emerald-950 text-emerald-100",
  warning: "border-amber-700 bg-amber-950 text-amber-100",
  error: "border-red-700 bg-red-950 text-red-100",
};

const AUTO_DISMISS_MS = 6000;

export function NotificationProvider({ children }: { children: ReactNode }) {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const nextId = useRef(0);

  const dismiss = useCallback((id: string) => {
    setNotifications((current) => current.filter((n) => n.id !== id));
  }, []);

  const notify = useCallback(
    (level: NotificationLevel, message: string) => {
      const id = `notification-${nextId.current++}`;
      setNotifications((current) => [...current, { id, level, message }]);
      window.setTimeout(() => dismiss(id), AUTO_DISMISS_MS);
    },
    [dismiss],
  );

  const value = useMemo(() => ({ notify, dismiss }), [notify, dismiss]);

  return (
    <NotificationContext.Provider value={value}>
      {children}
      <div
        role="region"
        aria-label="Notifications"
        className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-full max-w-sm flex-col gap-2"
      >
        {notifications.map((notification) => (
          <div
            key={notification.id}
            role="status"
            className={`pointer-events-auto flex items-start justify-between gap-3 rounded-md border px-3 py-2 text-sm shadow-lg ${LEVEL_STYLES[notification.level]}`}
          >
            <span>{notification.message}</span>
            <button
              type="button"
              onClick={() => dismiss(notification.id)}
              aria-label="Dismiss notification"
              className="shrink-0 rounded text-xs opacity-70 hover:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
            >
              ✕
            </button>
          </div>
        ))}
      </div>
    </NotificationContext.Provider>
  );
}
