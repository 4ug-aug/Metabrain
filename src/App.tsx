import { Routes, Route, NavLink } from "react-router-dom";
import { Chat, Settings } from "./pages";
import { MessageSquare, Settings as SettingsIcon, Brain } from "lucide-react";
import { cn } from "./lib/utils";
import { Toaster } from "./components/ui/sonner";

function App() {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background">
      {/* Sidebar Navigation */}
      <aside className="flex w-16 flex-col items-center border-r border-border bg-sidebar py-4">
        {/* Logo */}
        <div className="mb-8 flex h-10 w-10 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <Brain className="h-6 w-6" />
        </div>

        {/* Navigation Links */}
        <nav className="flex flex-1 flex-col gap-2">
          <NavLink
            to="/"
            className={({ isActive }) =>
              cn(
                "flex h-10 w-10 items-center justify-center rounded-lg transition-colors",
                isActive
                  ? "bg-accent text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              )
            }
          >
            <MessageSquare className="h-5 w-5" />
          </NavLink>
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              cn(
                "flex h-10 w-10 items-center justify-center rounded-lg transition-colors",
                isActive
                  ? "bg-accent text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              )
            }
          >
            <SettingsIcon className="h-5 w-5" />
          </NavLink>
        </nav>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-hidden">
        <Routes>
          <Route path="/" element={<Chat />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </main>

      {/* Toast Notifications */}
      <Toaster position="top-right" />
    </div>
  );
}

export default App;
