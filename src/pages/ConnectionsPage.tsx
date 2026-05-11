import { FormEvent, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, Connection, ConnectionInput } from "@/lib/api";
import { useActiveConnection } from "@/lib/store";
import { Banner, Button, Field, Input } from "@/components/ui";

const emptyInput: ConnectionInput = {
  name: "",
  endpoint: "localhost:50051",
  insecure: true,
  token: "",
};

export function ConnectionsPage() {
  const qc = useQueryClient();
  const { activeConnectionId, setActiveConnectionId } = useActiveConnection();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [input, setInput] = useState<ConnectionInput>(emptyInput);
  const [testResult, setTestResult] = useState<
    { kind: "ok" | "err"; message: string } | null
  >(null);

  const { data: connections = [], isLoading } = useQuery({
    queryKey: ["connections"],
    queryFn: api.listConnections,
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
      qc.invalidateQueries({ queryKey: ["connections"] });
      if (activeConnectionId === id) setActiveConnectionId(null);
    },
  });

  const testMut = useMutation({
    mutationFn: (id: string) => api.testConnection(id),
    onMutate: () => setTestResult(null),
    onSuccess: (msg) => setTestResult({ kind: "ok", message: msg }),
    onError: (err: Error) =>
      setTestResult({ kind: "err", message: err.message }),
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
    deleteMut.error?.message ||
    null;

  return (
    <div className="grid h-full grid-cols-[1fr_420px] gap-0 overflow-hidden">
      <section className="flex min-h-0 flex-col overflow-hidden border-r border-slate-200 dark:border-slate-800">
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 px-4 py-3">
          <h1 className="text-sm font-medium uppercase tracking-wide text-slate-600 dark:text-slate-400">
            Connections
          </h1>
          <Button
            variant="secondary"
            onClick={() => {
              setEditingId(null);
              setInput(emptyInput);
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
            <ul>
              {connections.map((c) => (
                <ConnectionRow
                  key={c.id}
                  conn={c}
                  isActive={c.id === activeConnectionId}
                  isEditing={c.id === editingId}
                  onSelect={() => setActiveConnectionId(c.id)}
                  onEdit={() => setEditingId(c.id)}
                  onDelete={() => {
                    if (confirm(`Delete connection "${c.name}"?`)) {
                      deleteMut.mutate(c.id);
                    }
                  }}
                  onTest={() => testMut.mutate(c.id)}
                  testing={testMut.isPending && testMut.variables === c.id}
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
      </section>

      <aside className="flex min-h-0 flex-col overflow-hidden">
        <div className="border-b border-slate-200 dark:border-slate-800 px-4 py-3">
          <h2 className="text-sm font-medium uppercase tracking-wide text-slate-600 dark:text-slate-400">
            {editingId ? "Edit connection" : "New connection"}
          </h2>
        </div>
        <form
          className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto p-4"
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
          <label className="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
            <input
              type="checkbox"
              checked={input.insecure}
              onChange={(e) =>
                setInput((p) => ({ ...p, insecure: e.target.checked }))
              }
            />
            Insecure (plaintext gRPC, no TLS)
          </label>

          {error && <Banner tone="error">{error}</Banner>}

          <div className="mt-2 flex gap-2">
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
}: {
  conn: Connection;
  isActive: boolean;
  isEditing: boolean;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onTest: () => void;
  testing: boolean;
}) {
  return (
    <li
      className={`flex items-center justify-between border-b border-slate-200/60 dark:border-slate-800/60 px-4 py-3 ${
        isEditing ? "bg-slate-100/60 dark:bg-slate-900/60" : ""
      }`}
    >
      <button
        onClick={onSelect}
        className="flex flex-col items-start text-left"
      >
        <div className="flex items-center gap-2">
          <span className="font-medium">{conn.name}</span>
          {isActive && (
            <span className="rounded bg-orange-500/20 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-orange-300">
              Active
            </span>
          )}
          {conn.insecure && (
            <span className="rounded bg-slate-200 dark:bg-slate-800 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-slate-600 dark:text-slate-400">
              Insecure
            </span>
          )}
        </div>
        <div className="text-xs text-slate-500">{conn.endpoint}</div>
      </button>
      <div className="flex gap-1">
        <Button variant="ghost" onClick={onTest} disabled={testing}>
          {testing ? "Testing…" : "Test"}
        </Button>
        <Button variant="ghost" onClick={onEdit}>
          Edit
        </Button>
        <Button variant="ghost" onClick={onDelete}>
          Delete
        </Button>
      </div>
    </li>
  );
}
