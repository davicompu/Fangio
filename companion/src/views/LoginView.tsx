import { useState } from "react";
import { login } from "../lib/api";

interface Props {
  onLogin: () => void;
}

export function LoginView({ onLogin }: Props) {
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await login(password);
      onLogin();
    } catch {
      setError("Incorrect password");
      setPassword("");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-full flex flex-col bg-[#0f1621] safe-top safe-bottom">
      {/* Top branding */}
      <div className="flex-1 flex flex-col items-center justify-center px-8 pb-6">
        <div className="mb-10 text-center">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-[var(--radius-xl)] bg-white/10 mb-5">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/>
              <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>
            </svg>
          </div>
          <h1 className="text-3xl font-semibold text-white tracking-tight">Reeves</h1>
          <p className="text-white/40 text-sm mt-1">Research companion</p>
        </div>

        {/* Card */}
        <div className="w-full max-w-sm bg-white/8 rounded-[var(--radius-xl)] p-6 space-y-4 border border-white/10">
          <p className="text-white/70 text-sm text-center">Enter your companion password</p>

          <form onSubmit={handleSubmit} className="space-y-3">
            <input
              type="password"
              value={password}
              onChange={(e) => { setPassword(e.target.value); setError(null); }}
              placeholder="Password"
              autoFocus
              autoComplete="current-password"
              className="w-full px-4 py-3.5 rounded-[var(--radius-md)] bg-white/10 border border-white/15 text-white placeholder:text-white/30 text-base outline-none focus:border-white/40 transition-colors"
            />

            {error && (
              <p className="text-[#f87171] text-sm text-center font-medium">{error}</p>
            )}

            <button
              type="submit"
              disabled={loading}
              className="w-full py-3.5 rounded-[var(--radius-md)] bg-white text-[var(--color-ink-800)] font-semibold text-base disabled:opacity-40 active:scale-[0.98] transition-transform"
            >
              {loading ? "Signing in…" : "Sign in"}
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}
