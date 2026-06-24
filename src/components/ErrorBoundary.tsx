import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  /** Shown in the fallback header so we know which area failed. */
  label?: string;
  children: ReactNode;
}
interface State {
  error: Error | null;
}

/** Catches render-time errors in a subtree so one broken panel can't blank the
 *  whole app. Shows the actual error message (so it's diagnosable) plus a button
 *  to retry rendering. */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Surface it in the console for diagnostics.
    console.error(`[ErrorBoundary${this.props.label ? ` · ${this.props.label}` : ""}]`, error, info);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <div className="m-6 max-w-2xl rounded-[var(--radius-md)] border border-[var(--color-warn-300)] bg-[var(--color-warn-50)] p-5">
        <div className="text-[13px] font-semibold text-[var(--color-warn-700)] mb-1">
          {this.props.label ? `${this.props.label} hit an error` : "This view hit an error"}
        </div>
        <p className="text-[12px] text-[var(--color-warn-700)]/90 leading-relaxed mb-3">
          The rest of the app is still fine. Here's what went wrong:
        </p>
        <pre className="text-[11px] font-mono text-[var(--color-ink-800)] bg-white/70 border border-[var(--color-warn-200)] rounded-[var(--radius-sm)] p-3 overflow-x-auto whitespace-pre-wrap">
          {error.message}
          {error.stack ? `\n\n${error.stack.split("\n").slice(0, 6).join("\n")}` : ""}
        </pre>
        <button
          onClick={() => this.setState({ error: null })}
          className="mt-3 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium bg-[var(--color-ink-800)] text-white hover:bg-[var(--color-ink-900)] transition-paper"
        >
          Try again
        </button>
      </div>
    );
  }
}
