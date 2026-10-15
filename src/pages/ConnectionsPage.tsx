import { FormEvent, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, Connection, ConnectionInput, TunnelStatus } from "@/lib/api";
import { useActiveConnection, useSchemaDrafts } from "@/lib/store";
import { Banner, Button, Dialog, Field, Input } from "@/components/ui";

const emptyInput: ConnectionInput = {
  name: "",
  endpoint: "localhost:50051",
  insecure: true,
  token: "",
  tunnel: null,
};

const defaultRailwayTunnel = {
  provider: "railway" as const,
  project: "",
  environment: "production",
  service: "",
  remote_port: 50051,
  local_port: 50051,
};

export function ConnectionsPage() {
  const qc = useQueryClient();
  const { activeConnectionId, setActiveConnectionId } = useActiveConnection();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Connection | null>(null);
  const [input, setInput] = useState<ConnectionInput>(emptyInput);
  const currentForm = useRef({ input, id: editingId });
  currentForm.current = { input, id: editingId };
  const [health, setHealth] = useState<Record<string, { ok: boolean; at: string }>>({});
  const [testResult, setTestResult] = useState<
    { kind: "ok" | "err"; message: string } | null
  >(null);
  const [formTestResult, setFormTestResult] = useState<
    { kind: "ok" | "err"; message: string } | null
  >(null);

  const { data: connections = [], isLoading } = useQuery({
    queryKey: ["connections"],
    queryFn: api.listConnections,
  });
  const { data: tunnelStatuses = [] } = useQuery({
    queryKey: ["tunnel-statuses"],
    queryFn: api.listTunnelStatuses,
    refetchInterval: 1000,
  });

  useEffect(() => {
    if (!editingId) {
      setInput(emptyInput);
      return;
    }
    const c = connections.find((x) => x.id === editingId);
    if (c) {
      setInput({
        name: c.name,
        endpoint: c.endpoint,
        insecure: c.insecure,
        token: "",
        tunnel: c.tunnel,
      });
    }
  }, [editingId, connections]);

  const createMut = useMutation({
    mutationFn: (i: ConnectionInput) => api.createConnection(i),
    onSuccess: (c) => {
      qc.invalidateQueries({ queryKey: ["connections"] });
      setEditingId(null);
      setInput(emptyInput);
      if (!activeConnectionId) setActiveConnectionId(c.id);
    },
  });

  const updateMut = useMutation({
    mutationFn: ({ id, input }: { id: string; input: ConnectionInput }) =>
      api.updateConnection(id, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["connections"] });
      setEditingId(null);
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => api.deleteConnection(id),
    onSuccess: (_, id) => {
      useSchemaDrafts.getState().removeDraft(id);
      qc.invalidateQueries({ queryKey: ["connections"] });
      if (activeConnectionId === id) setActiveConnectionId(null);
      setPendingDelete(null);
    },
  });

  const testMut = useMutation({
    mutationFn: (id: string) => api.testConnection(id),
    onMutate: () => setTestResult(null),
    onSuccess: (msg, id) => {
      setTestResult({ kind: "ok", message: `${connections.find(c => c.id === id)?.name ?? "Connection"}: ${msg}` });
      setHealth(h => ({ ...h, [id]: { ok: true, at: new Date().toLocaleTimeString() } }));
    },
    onError: (err: Error, id) => {
      setTestResult({ kind: "err", message: `${connections.find(c => c.id === id)?.name ?? "Connection"}: ${err.message}` });
      setHealth(h => ({ ...h, [id]: { ok: false, at: new Date().toLocaleTimeString() } }));
    },
  });

  const formTestMut = useMutation({
    mutationFn: ({ input, id }: { input: ConnectionInput; id: string | null }) =>
      api.testConnectionInput(input, id),
    onMutate: () => setFormTestResult(null),
    onSuccess: (msg, submitted) => {
      if (currentForm.current.input === submitted.input && currentForm.current.id === submitted.id) setFormTestResult({ kind: "ok", message: msg });
    },
    onError: (err: Error, submitted) => {
      if (currentForm.current.input === submitted.input && currentForm.current.id === submitted.id) setFormTestResult({ kind: "err", message: err.message });
    },
  });

  useEffect(() => {
    setFormTestResult(null);
    createMut.reset();
    updateMut.reset();
  }, [input, editingId]);

  const startTunnelMut = useMutation({
    mutationFn: api.startTunnel,
    onSettled: () => qc.invalidateQueries({ queryKey: ["tunnel-statuses"] }),
  });
  const stopTunnelMut = useMutation({
    mutationFn: api.stopTunnel,
    onSettled: () => qc.invalidateQueries({ queryKey: ["tunnel-statuses"] }),
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (editingId) {
      updateMut.mutate({ id: editingId, input });
    } else {
      createMut.mutate(input);
    }
  };

  const error =
    createMut.error?.message ||
    updateMut.error?.message ||
    null;

  return (
    <div className="connections-layout grid h-full grid-cols-[minmax(0,1fr)_minmax(320px,440px)] gap-0 overflow-hidden">
      <section className="flex min-h-0 flex-col overflow-hidden border-r border-slate-200 dark:border-slate-800">
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 px-6 py-5">
          <div>
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-orange-500">Network map</div>
            <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100">Connections</h1>
          </div>
          <Button
            variant="secondary"
            onClick={() => {
              setEditingId(null);
              setInput({ ...emptyInput });
              setFormTestResult(null);
              createMut.reset(); updateMut.reset();
            }}
          >
            New connection
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-auto">
          {isLoading ? (
            <div className="p-4 text-sm text-slate-500">Loading…</div>
          ) : connections.length === 0 ? (
            <div className="p-4 text-sm text-slate-500">
              No connections yet. Add one on the right to get started.
            </div>
          ) : (
            <ul className="space-y-2 p-3">
              {connections.map((c) => (
                <ConnectionRow
                  key={c.id}
                  conn={c}
                  isActive={c.id === activeConnectionId}
                  isEditing={c.id === editingId}
                  onSelect={() => setActiveConnectionId(c.id)}
                  onEdit={() => setEditingId(c.id)}
                  onDelete={() => { deleteMut.reset(); setPendingDelete(c); }}
                  health={health[c.id]}
                  onTest={() => testMut.mutate(c.id)}
                  testing={testMut.isPending && testMut.variables === c.id}
                  deleting={deleteMut.isPending && deleteMut.variables === c.id}
                  tunnelStatus={tunnelStatuses.find((status) => status.connection_id === c.id)}
                  onTunnelToggle={() => {
                    const status = tunnelStatuses.find((item) => item.connection_id === c.id);
                    if (status && status.state !== "stopped") stopTunnelMut.mutate(c.id);
                    else startTunnelMut.mutate(c.id);
                  }}
                />
              ))}
            </ul>
          )}
        </div>
        {testResult && (
          <div className="border-t border-slate-200 dark:border-slate-800 p-3">
            <Banner tone={testResult.kind === "ok" ? "success" : "error"}>
              {testResult.message}
            </Banner>
          </div>
        )}
        {(startTunnelMut.error || stopTunnelMut.error) && <div className="p-3"><Banner tone="error">{startTunnelMut.error?.message || stopTunnelMut.error?.message}</Banner></div>}
      </section>

      <aside className="flex min-h-0 flex-col overflow-hidden">
        <div className="border-b border-slate-200 dark:border-slate-800 px-5 py-5">
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-orange-500">Configuration</div>
          <h2 className="text-base font-bold text-slate-900 dark:text-slate-100">
            {editingId ? "Edit connection" : "New connection"}
          </h2>
        </div>
        <form
          className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto p-5"
          onSubmit={onSubmit}
        >
          <Field label="Name">
            <Input
              required
              autoFocus
              value={input.name}
              onChange={(e) =>
                setInput((p) => ({ ...p, name: e.target.value }))
              }
              placeholder="Local SpiceDB"
            />
          </Field>
          <Field label="Connection method">
            <select
              className="w-full rounded border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-900 px-2 py-1.5 text-sm"
              value={input.tunnel?.provider ?? "direct"}
              onChange={(event) =>
                setInput((current) => ({
                  ...current,
                  tunnel: event.target.value === "railway" ? defaultRailwayTunnel : null,
                }))
              }
            >
              <option value="direct">Direct endpoint</option>
              <option value="railway">Railway private service</option>
            </select>
          </Field>

          {input.tunnel ? (
            <div className="space-y-3 rounded-xl border border-orange-500/20 bg-orange-500/[0.035] p-4">
              <div>
                <div className="text-sm font-medium">Railway SSH tunnel</div>
                <div className="mt-1 text-xs text-slate-500">
                  Uses your Railway CLI login and SSH key. SpiceDB remains private.
                </div>
              </div>
              <Field label="Project ID" hint="Find this in Railway project settings.">
                <Input
                  required
                  value={input.tunnel.project}
                  onChange={(event) => setInput((current) => current.tunnel ? ({ ...current, tunnel: { ...current.tunnel, project: event.target.value } }) : current)}
                  placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                />
              </Field>
              <div className="grid grid-cols-2 gap-3">
                <Field label="Environment">
                  <Input
                    required
                    value={input.tunnel.environment}
                    onChange={(event) => setInput((current) => current.tunnel ? ({ ...current, tunnel: { ...current.tunnel, environment: event.target.value } }) : current)}
                    placeholder="production"
                  />
                </Field>
                <Field label="Service">
                  <Input
                    required
                    value={input.tunnel.service}
                    onChange={(event) => setInput((current) => current.tunnel ? ({ ...current, tunnel: { ...current.tunnel, service: event.target.value } }) : current)}
                    placeholder="SpiceDB"
                  />
                </Field>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <Field label="SpiceDB port">
                  <Input
                    required
                    type="number"
                    min={1}
                    max={65535}
                    value={input.tunnel.remote_port}
                    onChange={(event) => setInput((current) => current.tunnel ? ({ ...current, tunnel: { ...current.tunnel, remote_port: Number(event.target.value) } }) : current)}
                  />
                </Field>
                <Field label="Local port" hint="Choose another if this port is already in use.">
                  <Input
                    required
                    type="number"
                    min={1}
                    max={65535}
                    value={input.tunnel.local_port}
                    onChange={(event) => setInput((current) => current.tunnel ? ({ ...current, tunnel: { ...current.tunnel, local_port: Number(event.target.value) } }) : current)}
                  />
                </Field>
              </div>
              <div className="text-xs text-slate-500">
                Local endpoint: 127.0.0.1:{input.tunnel.local_port} · plaintext is protected inside the SSH tunnel
              </div>
            </div>
          ) : (
            <Field
              label="Endpoint"
              hint="host:port — e.g. localhost:50051 or grpc.authzed.com:443"
            >
              <Input
                required
                value={input.endpoint}
                onChange={(e) =>
                  setInput((p) => ({ ...p, endpoint: e.target.value }))
                }
                placeholder="localhost:50051"
              />
            </Field>
          )}
          <Field
            label="Pre-shared key (token)"
            hint={
              editingId
                ? "Leave blank to keep the current token."
                : "Stored in your OS keychain."
            }
          >
            <Input
              type="password"
              value={input.token}
              onChange={(e) =>
                setInput((p) => ({ ...p, token: e.target.value }))
              }
              placeholder={editingId ? "(unchanged)" : "somerandomkeyhere"}
            />
          </Field>
          {!input.tunnel && <label className="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
            <input
              type="checkbox"
              checked={input.insecure}
              onChange={(e) =>
                setInput((p) => ({ ...p, insecure: e.target.checked }))
              }
            />
            Insecure (plaintext gRPC, no TLS)
          </label>}

          {error && <Banner tone="error">{error}</Banner>}
          {formTestResult && (
            <Banner tone={formTestResult.kind === "ok" ? "success" : "error"}>
              {formTestResult.message}
            </Banner>
          )}

          <div className="mt-2 flex flex-wrap gap-2">
            <Button
              type="button"
              variant="secondary"
              disabled={formTestMut.isPending}
              onClick={() => formTestMut.mutate({ input, id: editingId })}
            >
              {formTestMut.isPending ? "Testing…" : "Test connection"}
            </Button>
            <Button
              type="submit"
              disabled={createMut.isPending || updateMut.isPending}
            >
              {editingId ? "Save changes" : "Create connection"}
            </Button>
            {editingId && (
              <Button
                type="button"
                variant="ghost"
                onClick={() => setEditingId(null)}
              >
                Cancel
              </Button>
            )}
          </div>
        </form>
      </aside>
      {pendingDelete && (
        <Dialog title="Delete connection?" busy={deleteMut.isPending} onClose={() => setPendingDelete(null)}>
            <p className="mt-2 text-sm text-slate-600 dark:text-slate-400">
              Delete <span className="font-medium text-slate-900 dark:text-slate-100">{pendingDelete.name}</span> and its saved keychain credential from this Mac?
            </p>
            {pendingDelete.tunnel && (
              <p className="mt-2 text-xs text-slate-500">Its Railway tunnel will be stopped first.</p>
            )}
            {deleteMut.error && <Banner tone="error">{deleteMut.error.message}</Banner>}
            <div className="mt-5 flex justify-end gap-2">
              <Button variant="secondary" disabled={deleteMut.isPending} onClick={() => setPendingDelete(null)}>
                Cancel
              </Button>
              <Button variant="danger" disabled={deleteMut.isPending} onClick={() => deleteMut.mutate(pendingDelete.id)}>
                {deleteMut.isPending ? "Deleting…" : "Delete connection"}
              </Button>
            </div>
        </Dialog>
      )}
    </div>
  );
}

function ConnectionRow({
  conn,
  isActive,
  isEditing,
  onSelect,
  onEdit,
  onDelete,
  onTest,
  testing,
  deleting,
  tunnelStatus,
  onTunnelToggle,
  health,
}: {
  conn: Connection;
  isActive: boolean;
  isEditing: boolean;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onTest: () => void;
  testing: boolean;
  deleting: boolean;
  tunnelStatus?: TunnelStatus;
  onTunnelToggle: () => void;
  health?: { ok: boolean; at: string };
}) {
  return (
    <li
      className={`flex flex-wrap items-center justify-between gap-3 rounded-xl border px-4 py-4 transition ${
        isEditing
          ? "border-orange-500/40 bg-orange-500/[0.06]"
          : "border-slate-200/70 bg-white/40 hover:border-slate-300 dark:border-slate-800/80 dark:bg-slate-950/35 dark:hover:border-slate-700"
      }`}
    >
      <button
        onClick={onSelect}
        className="flex min-w-0 flex-1 flex-col items-start text-left"
      >
        <div className="flex flex-wrap items-center gap-2">
          <span className="break-words font-medium">{conn.name}</span>
          {isActive && (
            <span className="rounded bg-orange-500/20 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-orange-800 dark:text-orange-300">
              Active
            </span>
          )}
          {conn.insecure && !conn.tunnel && (
            <span className="rounded bg-slate-200 dark:bg-slate-800 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-slate-600 dark:text-slate-400">
              Plaintext
            </span>
          )}
          {conn.tunnel && (
            <span className={`rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide ${
              tunnelStatus?.state === "connected"
                ? "bg-emerald-500/20 text-emerald-600 dark:text-emerald-300"
                : tunnelStatus?.state === "reconnecting"
                  ? "bg-amber-500/20 text-amber-600 dark:text-amber-300"
                  : "bg-slate-200 dark:bg-slate-800 text-slate-600 dark:text-slate-400"
            }`} title={tunnelStatus?.message ?? undefined}>
              SSH tunnel · {tunnelStatus?.state ?? "stopped"}
            </span>
          )}
        </div>
        <div className="break-all text-xs text-slate-500">{conn.endpoint}</div>
        <div className={`mt-1 text-xs ${health?.ok ? "text-emerald-700 dark:text-emerald-300" : "text-slate-600 dark:text-slate-400"}`}>
          {health ? `Last SpiceDB test: ${health.ok ? "reachable" : "failed"} at ${health.at}` : "SpiceDB not tested this session"}
        </div>
        {conn.tunnel && tunnelStatus?.message && tunnelStatus.state !== "connected" && (
          <div className="mt-1 max-w-md text-xs text-amber-600 dark:text-amber-400">
            {tunnelStatus.message}
          </div>
        )}
      </button>
      <div className="flex flex-wrap gap-1">
        {conn.tunnel && (
          <Button variant="ghost" onClick={onTunnelToggle}>
            {tunnelStatus && tunnelStatus.state !== "stopped" ? "Stop tunnel" : "Start tunnel"}
          </Button>
        )}
        <Button variant="ghost" onClick={onTest} disabled={testing}>
          {testing ? "Testing…" : "Test"}
        </Button>
        <Button variant="ghost" onClick={onEdit}>
          Edit
        </Button>
        <Button variant="ghost" onClick={(event) => { event.currentTarget.focus(); onDelete(); }} disabled={deleting}>
          {deleting ? "Deleting…" : "Delete"}
        </Button>
      </div>
    </li>
  );
}
