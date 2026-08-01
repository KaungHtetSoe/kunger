import { Component, type ErrorInfo, type ReactNode } from "react";
import { logger } from "@/lib/logger";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    logger.error("Unhandled error in application shell", {
      message: error.message,
      componentStack: info.componentStack,
    });
  }

  private handleReset = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    if (this.state.error) {
      return (
        <div
          role="alert"
          className="flex h-screen w-screen flex-col items-center justify-center gap-4 bg-neutral-950 p-8 text-neutral-100"
        >
          <h1 className="text-lg font-semibold">Kunger hit an unexpected error</h1>
          <p className="max-w-md text-center text-sm text-neutral-400">
            {this.state.error.message || "An unknown error occurred in the application shell."}
          </p>
          <button
            type="button"
            onClick={this.handleReset}
            className="rounded-md border border-neutral-700 px-4 py-2 text-sm hover:bg-neutral-800"
          >
            Try again
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
