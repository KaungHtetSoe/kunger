interface ErrorStateProps {
  message: string;
  onRetry?: () => void;
}

/** For a failed data fetch (e.g. a command call), distinct from the
 * app-level ErrorBoundary, which only catches render-time exceptions. */
export function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div
      role="alert"
      className="flex h-full flex-col items-center justify-center gap-3 p-12 text-center"
    >
      <p className="text-sm font-medium text-red-400">Something went wrong</p>
      <p className="max-w-sm text-sm text-neutral-500">{message}</p>
      {onRetry && (
        <button
          type="button"
          onClick={onRetry}
          className="mt-2 rounded-md border border-neutral-700 px-3 py-1.5 text-sm hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        >
          Try again
        </button>
      )}
    </div>
  );
}
