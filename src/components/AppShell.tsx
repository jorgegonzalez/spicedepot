import { useEffect } from "react";
import appIcon from "@/assets/spicedepot-icon.png";
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
  { to: "/mcp", label: "MCP" },
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
    <div className="ginyu-app flex h-full flex-col bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100">
      <header className="app-header flex items-center justify-between gap-4 px-4 py-3">
        <div className="app-primary flex min-w-0 items-center gap-5">
          <div className="app-brand flex shrink-0 items-center gap-2">
            <img src={appIcon} alt="" className="h-10 w-10 shrink-0 object-contain" />
            <div>
              <span className="brand-display block text-[13px] font-bold leading-none">SpiceDepot</span>
              <span className="mt-1 block text-[9px] font-semibold uppercase tracking-[0.18em] text-orange-400">Permission systems</span>
            </div>
          </div>
          <nav className="app-nav flex gap-0.5">
            {navItems.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  `app-nav-link px-3 py-1.5 text-sm transition ${
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
        <div className="app-connection flex min-w-0 items-center gap-2">
          <label htmlFor="active-connection" className="text-[10px] font-semibold uppercase tracking-[0.12em] text-slate-500">Active</label>
          <select
            id="active-connection"
            className="connection-control max-w-[220px] border border-slate-300 dark:border-slate-700 px-3 py-1.5 text-sm outline-none focus:border-orange-500"
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
            <span className="app-endpoint text-xs text-slate-500">{active.endpoint}</span>
          )}
          <ThemeToggle />
        </div>
      </header>
      <main className="ginyu-main min-h-0 flex-1 overflow-hidden">
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
      className="theme-control ml-1 inline-flex h-9 w-9 items-center justify-center border border-slate-300 dark:border-slate-700 text-slate-600 dark:text-slate-400 transition hover:border-orange-500/60 hover:text-orange-400"
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
    <footer className="app-footer flex items-center justify-between gap-4 px-2 py-2 text-[11px] text-slate-500">
      <div>
        <span>SpiceDepot · open source (Apache 2.0)</span>
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
