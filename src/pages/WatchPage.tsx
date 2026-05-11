import { useCallback, useEffect, useRef, useState } from "react";
import { api, WatchEvent } from "@/lib/api";
import { Channel } from "@/lib/tauri";
import { useActiveConnection } from "@/lib/store";
import { Banner, Button, Input } from "@/components/ui";

const MAX_EVENTS = 1000;

/** What we render: WatchEvent plus the client-side receive time. */
type LogRow = WatchEvent & { receivedAt: number; id: number };

export function WatchPage() {
  const { activeConnectionId } = useActiveConnection();
  const [objectTypesText, setObjectTypesText] = useState("");
  const [log, setLog] = useState<LogRow[]>([]);
  const [watchId, setWatchId] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const counter = useRef(0);

  // Stop any active watch when the page unmounts or the active connection changes.
  useEffect(() => {
    return () => {
      if (watchId) {
        api.watchStop(watchId).catch(() => {
          // Best-effort: server task may have already exited.
        });
      }
    };
    // We deliberately stop only on unmount / id transition.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // When the active connection changes, stop the current watch and clear the log.
  useEffect(() => {
    if (watchId) {
      api.watchStop(watchId).catch(() => {});
      setWatchId(null);
      setLog([]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeConnectionId]);

  const start = useCallback(async () => {
    if (!activeConnectionId) return;
    setStartError(null);
    setStarting(true);

    const channel = new Channel<WatchEvent>();
    channel.onmessage = (evt) => {
      setLog((prev) => {
        const next: LogRow[] = [
          { ...evt, receivedAt: Date.now(), id: ++counter.current },
          ...prev,
        ];
        return next.length > MAX_EVENTS ? next.slice(0, MAX_EVENTS) : next;
      });
    };

    const object_types = objectTypesText
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    try {
      const id = await api.watchStart(
        activeConnectionId,
        { object_types },
        channel,
      );
      setWatchId(id);
    } catch (e) {
      setStartError((e as Error).message);
    } finally {
      setStarting(false);
    }
  }, [activeConnectionId, objectTypesText]);

  const stop = useCallback(async () => {
    if (!watchId) return;
    try {
      await api.watchStop(watchId);
    } catch {
      // Server task may have already exited; harmless.
    } finally {
      setWatchId(null);
    }
  }, [watchId]);

  if (!activeConnectionId) {
    return (
      <div className="p-6 text-sm text-slate-400">
        Select a connection from the header to watch relationship changes.
      </div>
    );
  }

  const isRunning = !!watchId;

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden">
      <div className="flex flex-wrap items-end gap-3 border-b border-slate-800 p-4">
        <div className="min-w-[280px] flex-1">
          <label className="mb-1 block text-xs uppercase tracking-wide text-slate-400">
            Object types (comma-separated, blank = all)
          </label>
          <Input
            value={objectTypesText}
            onChange={(e) => setObjectTypesText(e.target.value)}
            placeholder="document, group"
            disabled={isRunning}
          />
        </div>
        <div className="flex gap-2">
          {isRunning ? (
            <Button variant="danger" onClick={stop}>
              ● Stop
            </Button>
          ) : (
            <Button onClick={start} disabled={starting}>
              {starting ? "Starting…" : "▶ Start watching"}
            </Button>
          )}
          <Button
            variant="ghost"
            onClick={() => setLog([])}
            disabled={log.length === 0}
          >
            Clear log
          </Button>
        </div>
      </div>

      {startError && (
        <div className="p-3">
          <Banner tone="error">{startError}</Banner>
        </div>
      )}

      <StatusBar isRunning={isRunning} count={log.length} />

      <div className="min-h-0 flex-1 overflow-auto font-mono text-xs">
        {log.length === 0 && (
          <div className="p-4 text-slate-500">
            {isRunning
              ? "Watching for relationship changes. Events will appear here as they happen."
              : "Press Start watching to begin a live stream."}
          </div>
        )}
        {log.map((row) => (
          <LogLine key={row.id} row={row} />
        ))}
      </div>
    </div>
  );
}

function StatusBar({
  isRunning,
  count,
}: {
  isRunning: boolean;
  count: number;
}) {
  return (
    <div className="flex items-center gap-3 border-b border-slate-800 px-4 py-2 text-xs">
      <div
        className={`flex items-center gap-1.5 ${
          isRunning ? "text-emerald-300" : "text-slate-500"
        }`}
      >
        <span
          className={`inline-block h-2 w-2 rounded-full ${
            isRunning ? "animate-pulse bg-emerald-400" : "bg-slate-600"
          }`}
        />
        {isRunning ? "Live" : "Idle"}
      </div>
      <span className="text-slate-500">
        {count} {count === 1 ? "event" : "events"}
        {count >= MAX_EVENTS && ` (cap: oldest dropped)`}
      </span>
    </div>
  );
}

function LogLine({ row }: { row: LogRow }) {
  const time = new Date(row.receivedAt).toLocaleTimeString();
  switch (row.kind) {
    case "update": {
      const op = row.operation;
      const tone =
        op === "create"
          ? "text-emerald-300"
          : op === "touch"
          ? "text-sky-300"
          : "text-red-300";
      const r = row.relationship;
      const subject = `${r.subject_type}:${r.subject_id}${
        r.subject_relation ? `#${r.subject_relation}` : ""
      }`;
      return (
        <div className="flex gap-3 border-b border-slate-900 px-4 py-1.5 hover:bg-slate-900/40">
          <span className="text-slate-500">{time}</span>
          <span className={`w-12 uppercase ${tone}`}>{op}</span>
          <span className="text-slate-200">
            {r.resource_type}:{r.resource_id}#{r.relation} @ {subject}
          </span>
        </div>
      );
    }
    case "schema_changed":
      return (
        <div className="flex gap-3 border-b border-slate-900 px-4 py-1.5 text-amber-300">
          <span className="text-slate-500">{time}</span>
          <span className="w-12 uppercase">schema</span>
          <span>schema updated{row.at && ` (at ${row.at})`}</span>
        </div>
      );
    case "ended":
      return (
        <div className="flex gap-3 border-b border-slate-900 px-4 py-1.5 text-slate-500">
          <span>{time}</span>
          <span className="w-12 uppercase">end</span>
          <span>stream ended</span>
        </div>
      );
    case "error":
      return (
        <div className="flex gap-3 border-b border-slate-900 px-4 py-1.5 text-red-300">
          <span className="text-slate-500">{time}</span>
          <span className="w-12 uppercase">error</span>
          <span>{row.message}</span>
        </div>
      );
  }
}
