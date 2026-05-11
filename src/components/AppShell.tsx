import { useEffect } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "@/lib/api";
import { useActiveConnection, useTheme } from "@/lib/store";
import {
  BUY_ME_A_COFFEE_URL,
  GITHUB_SPONSORS_URL,
} from "@/lib/support";

const navItems = [
  { to: "/connections", label: "Connections" },
  { to: "/schema", label: "Schema" },
  { to: "/relationships", label: "Relationships" },
  { to: "/permissions", label: "Check" },
  { to: "/lookup", label: "Lookup" },
  { to: "/watch", label: "Watch" },
];

export function AppShell() {
  const navigate = useNavigate();
  const { activeConnectionId, setActiveConnectionId } = useActiveConnection();
  const { data: connections } = useQuery({
    queryKey: ["connections"],
    queryFn: api.listConnections,
  });

  // The active connection id is persisted in localStorage and can outlive the
  // connection itself (deleted in a previous session, or wiped from the JSON
  // store manually). Drop it if it no longer matches a real connection — but
  // only once the connection list has actually loaded.
  useEffect(() => {
    if (
      connections &&
      activeConnectionId &&
      !connections.some((c) => c.id === activeConnectionId)
    ) {
      setActiveConnectionId(null);
    }
  }, [connections, activeConnectionId, setActiveConnectionId]);

  const active =
    connections?.find((c) => c.id === activeConnectionId) ?? null;

  return (
    <div className="flex h-full flex-col bg-white dark:bg-slate-950 text-slate-900 dark:text-slate-100">
      <header className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 px-4 py-2">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <div className="h-2 w-2 rounded-full bg-orange-500" />
            <span className="font-semibold tracking-tight">SpiceLens</span>
          </div>
          <nav className="flex gap-1">
            {navItems.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  `rounded px-3 py-1.5 text-sm transition ${
                    isActive
                      ? "bg-slate-200 dark:bg-slate-800 text-slate-900 dark:text-slate-100"
                      : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-900 hover:text-slate-800 dark:hover:text-slate-200"
                  }`
                }
              >
                {item.label}
              </NavLink>
            ))}
          </nav>
        </div>
        <div className="flex items-center gap-2">
          <label className="text-xs text-slate-500">Active:</label>
          <select
            className="rounded border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-900 px-2 py-1 text-sm"
            value={activeConnectionId ?? ""}
            onChange={(e) => {
              const id = e.target.value || null;
              setActiveConnectionId(id);
              if (id) navigate("/schema");
            }}
          >
            <option value="">— none —</option>
            {connections?.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          {active && (
            <span className="text-xs text-slate-500">{active.endpoint}</span>
          )}
          <ThemeToggle />
        </div>
      </header>
      <main className="min-h-0 flex-1 overflow-hidden">
        <Outlet />
      </main>
      <SupportFooter />
    </div>
  );
}

function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  const isDark = theme === "dark";
  return (
    <button
      onClick={toggleTheme}
      title={isDark ? "Switch to light theme" : "Switch to dark theme"}
      aria-label="Toggle theme"
      className="ml-1 inline-flex h-7 w-7 items-center justify-center rounded border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-900 text-slate-600 dark:text-slate-400 transition hover:bg-slate-200 dark:hover:bg-slate-800"
    >
      {isDark ? (
        // sun
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></svg>
      ) : (
        // moon
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
      )}
    </button>
  );
}

function SupportFooter() {
  const open = (url: string) => () => {
    // Tauri plugin-opener fires the system "open URL" handler, which lands
    // in Safari / Chrome / Firefox depending on the user's default.
    openUrl(url).catch(() => {
      // Last-ditch fallback if the opener plugin isn't available for some
      // reason — most browsers will at least navigate to the URL.
      window.open(url, "_blank", "noopener,noreferrer");
    });
  };
  return (
    <footer className="flex items-center justify-between gap-4 border-t border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 px-4 py-1.5 text-xs text-slate-500">
      <div>
        <span>SpiceLens · open source (MIT)</span>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-slate-400 dark:text-slate-600">Like this tool?</span>
        <button
          onClick={open(GITHUB_SPONSORS_URL)}
          className="rounded px-1.5 py-0.5 transition hover:bg-slate-200 dark:hover:bg-slate-800 hover:text-slate-800 dark:hover:text-slate-200"
        >
          ♥ Sponsor
        </button>
        <button
          onClick={open(BUY_ME_A_COFFEE_URL)}
          className="rounded px-1.5 py-0.5 transition hover:bg-slate-200 dark:hover:bg-slate-800 hover:text-slate-800 dark:hover:text-slate-200"
        >
          ☕ Buy me a coffee
        </button>
      </div>
    </footer>
  );
}
