import { useState } from "react";
import { isLoggedIn, clearToken } from "./lib/api";
import { LoginView } from "./views/LoginView";
import { HomeView } from "./views/HomeView";
import { ScreeningView } from "./views/ScreeningView";
import { ArticlesView } from "./views/ArticlesView";
import { CorpusChatView } from "./views/CorpusChatView";

type View = "home" | "chat" | "screening" | "articles";

export default function App() {
  const [loggedIn, setLoggedIn] = useState(isLoggedIn);
  const [view, setView] = useState<View>("home");

  function handleLogout() {
    clearToken();
    setLoggedIn(false);
    setView("home");
  }

  if (!loggedIn) {
    return <LoginView onLogin={() => setLoggedIn(true)} />;
  }

  return (
    <div className="flex flex-col h-full bg-[var(--color-paper-50)] safe-top safe-bottom">
      {view === "home" && (
        <HomeView
          onNavigate={setView}
          onLogout={handleLogout}
        />
      )}
      {view === "chat" && <CorpusChatView onBack={() => setView("home")} />}
      {view === "screening" && <ScreeningView onBack={() => setView("home")} />}
      {view === "articles" && <ArticlesView onBack={() => setView("home")} />}
    </div>
  );
}
