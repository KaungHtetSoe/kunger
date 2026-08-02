interface LoadingStateProps {
  label?: string;
}

export function LoadingState({ label = "Loading…" }: LoadingStateProps) {
  return (
    <div role="status" className="flex h-full flex-col items-center justify-center gap-3 p-12">
      <div
        aria-hidden="true"
        className="h-6 w-6 animate-spin rounded-full border-2 border-neutral-700 border-t-neutral-300"
      />
      <p className="text-sm text-neutral-500">{label}</p>
    </div>
  );
}
