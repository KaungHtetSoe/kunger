/**
 * Minimal frontend logging wrapper.
 *
 * Kept as a single indirection point so a real sink (e.g. forwarding to the
 * Rust side via `@tauri-apps/plugin-log`) can be added later without
 * touching call sites throughout the app.
 */

type LogLevel = "debug" | "info" | "warn" | "error";

function log(level: LogLevel, message: string, context?: Record<string, unknown>): void {
  const entry = context ? [message, context] : [message];
  console[level](`[kunger]`, ...entry);
}

export const logger = {
  debug: (message: string, context?: Record<string, unknown>) => log("debug", message, context),
  info: (message: string, context?: Record<string, unknown>) => log("info", message, context),
  warn: (message: string, context?: Record<string, unknown>) => log("warn", message, context),
  error: (message: string, context?: Record<string, unknown>) => log("error", message, context),
};
