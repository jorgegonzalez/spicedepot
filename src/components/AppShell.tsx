import { useEffect } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "@/lib/api";
import { useActiveConnection } from "@/lib/store";
import {
  BUY_ME_A_COFFEE_URL,
  GITHUB_SPONSORS_URL,
  PROBE_SUPPORT_URL,
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
    <div className="flex h-full flex-col bg-slate-950 text-slate-100">
      <header className="flex items-center justify-between border-b border-slate-800 px-4 py-2">
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
                      ? "bg-slate-800 text-slate-100"
                      : "text-slate-400 hover:bg-slate-900 hover:text-slate-200"
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
            className="rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm"
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
        </div>
      </header>
      <main className="min-h-0 flex-1 overflow-hidden">
        <Outlet />
      </main>
      <SupportFooter />
    </div>
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
    <footer className="flex items-center justify-between gap-4 border-t border-slate-800 bg-slate-950 px-4 py-1.5 text-xs text-slate-500">
      <div>
        <span>SpiceLens · open source (MIT)</span>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-slate-600">Like this tool?</span>
        <button
          onClick={open(GITHUB_SPONSORS_URL)}
          className="rounded px-1.5 py-0.5 transition hover:bg-slate-800 hover:text-slate-200"
        >
          ♥ Sponsor
        </button>
        <button
          onClick={open(BUY_ME_A_COFFEE_URL)}
          className="rounded px-1.5 py-0.5 transition hover:bg-slate-800 hover:text-slate-200"
        >
          ☕ Buy me a coffee
        </button>
        <span className="text-slate-700">·</span>
        <button
          onClick={open(PROBE_SUPPORT_URL)}
          className="rounded px-1.5 py-0.5 transition hover:bg-slate-800 hover:text-slate-200"
          title="Paid support via Probe"
        >
          Paid support →
        </button>
      </div>
    </footer>
  );
}
