import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { Banner, Button } from "@/components/ui";

type Agent = "claude" | "codex";

export function McpPage() {
  const queryClient = useQueryClient();
  const [connecting, setConnecting] = useState<Agent | null>(null);
  const [message, setMessage] = useState<{ tone: "success" | "error"; text: string } | null>(null);
  const { data: activity = [], isLoading: activityLoading } = useQuery({
    queryKey: ["mcp-activity"],
    queryFn: api.listMcpActivity,
    refetchInterval: 1500,
  });
  const clearActivity = useMutation({
    mutationFn: api.clearMcpActivity,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["mcp-activity"] }),
  });

  async function connect(agent: Agent) {
    setConnecting(agent);
    setMessage(null);
    try {
      const result = await api.connectMcpClient(agent);
      setMessage({ tone: "success", text: result.message });
    } catch (error) {
      setMessage({ tone: "error", text: String(error) });
    } finally {
      setConnecting(null);
    }
  }

  return (
    <div className="h-full overflow-auto p-6">
      <div className="mx-auto max-w-4xl space-y-6">
        <div>
          <h1 className="text-xl font-semibold">Connect an agent</h1>
          <p className="mt-1 text-sm text-slate-600 dark:text-slate-400">
            Choose an agent to add SpiceDepot to its MCP settings. After setup, open SpiceDepot before starting or restarting the agent so it can connect.
          </p>
        </div>

        <Banner tone="info">
          The agent starts its local connection when it launches. SpiceDepot must already be open; opening SpiceDepot does not push a connection into an agent that is already running. Credentials stay in SpiceDepot and the OS keychain, and the app does not open a network port. MCP tools are read-only and cannot change your SpiceDB data.
        </Banner>

        {message && <Banner tone={message.tone}>{message.text}</Banner>}

        <div className="grid gap-4 sm:grid-cols-2">
          <AgentCard
            title="Claude Code"
            description="Add SpiceDepot to your user-level Claude Code MCP settings."
            busy={connecting === "claude"}
            disabled={connecting !== null}
            onConnect={() => void connect("claude")}
          />
          <AgentCard
            title="Codex"
            description="Add SpiceDepot to your Codex MCP settings."
            busy={connecting === "codex"}
            disabled={connecting !== null}
            onConnect={() => void connect("codex")}
          />
        </div>

        <section className="rounded border border-slate-200 dark:border-slate-800 p-4">
          <h2 className="font-medium">Available read-only tools</h2>
          <ul className="mt-3 grid gap-2 text-sm text-slate-600 dark:text-slate-400 sm:grid-cols-2">
            <Tool name="list_connections" detail="List saved connection names and endpoints; no tokens." />
            <Tool name="read_schema" detail="Read the schema from a selected connection." />
            <Tool name="check_permission" detail="Check if a subject has a permission on a resource." />
            <Tool name="lookup_resources" detail="Find resources a subject can access (up to 500)." />
            <Tool name="lookup_subjects" detail="Find subjects with access to a resource (up to 500)." />
            <Tool name="read_relationships" detail="Read matching relationships (up to 500)." />
          </ul>
          <p className="mt-4 text-xs text-slate-500">
            Select a saved connection by name or ID when asking an agent to use these tools. Restart the agent app if it does not show SpiceDepot immediately.
          </p>
        </section>

        <section className="overflow-hidden rounded border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 px-4 py-3">
            <div>
              <h2 className="font-medium">MCP activity</h2>
              <p className="mt-0.5 text-xs text-slate-500">
                The latest 500 tool calls are stored locally. Logs include resource and subject IDs; avoid putting secrets in query fields.
              </p>
            </div>
            <Button
              variant="secondary"
              disabled={activity.length === 0 || clearActivity.isPending}
              onClick={() => clearActivity.mutate()}
            >
              {clearActivity.isPending ? "Clearing…" : "Clear log"}
            </Button>
          </div>
          {activityLoading ? (
            <div className="p-4 text-sm text-slate-500">Loading activity…</div>
          ) : activity.length === 0 ? (
            <div className="p-4 text-sm text-slate-500">
              No MCP requests yet. Agent tool calls will appear here automatically.
            </div>
          ) : (
            <div className="max-h-[420px] overflow-auto">
              <table className="w-full text-left text-sm">
                <thead className="sticky top-0 bg-slate-100 dark:bg-slate-900 text-xs uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-3 py-2 font-medium">Time</th>
                    <th className="px-3 py-2 font-medium">Tool</th>
                    <th className="px-3 py-2 font-medium">Connection</th>
                    <th className="px-3 py-2 font-medium">Result</th>
                    <th className="px-3 py-2 text-right font-medium">Duration</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-200 dark:divide-slate-800">
                  {activity.map((entry) => (
                    <tr key={entry.id} className="align-top">
                      <td className="whitespace-nowrap px-3 py-2 text-xs text-slate-500">
                        {new Date(entry.requested_at).toLocaleString()}
                      </td>
                      <td className="px-3 py-2">
                        <details>
                          <summary className="cursor-pointer font-mono text-xs text-slate-900 dark:text-slate-100">
                            {entry.tool}
                          </summary>
                          <pre className="mt-2 max-w-md overflow-auto whitespace-pre-wrap break-all rounded bg-slate-100 dark:bg-slate-900 p-2 text-[11px] text-slate-600 dark:text-slate-400">
                            {JSON.stringify(entry.arguments, null, 2)}
                          </pre>
                        </details>
                      </td>
                      <td className="px-3 py-2">
                        <div>{entry.connection_name ?? "—"}</div>
                        {entry.endpoint && <div className="text-xs text-slate-500">{entry.endpoint}</div>}
                      </td>
                      <td className="px-3 py-2">
                        <span className={entry.status === "success" ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}>
                          {entry.status === "success" ? "Success" : "Error"}
                        </span>
                        {(entry.summary || entry.error) && (
                          <div className="mt-0.5 max-w-sm break-words text-xs text-slate-500">
                            {entry.summary ?? entry.error}
                          </div>
                        )}
                      </td>
                      <td className="whitespace-nowrap px-3 py-2 text-right text-xs text-slate-500">
                        {entry.duration_ms} ms
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

function AgentCard({
  title,
  description,
  busy,
  disabled,
  onConnect,
}: {
  title: string;
  description: string;
  busy: boolean;
  disabled: boolean;
  onConnect: () => void;
}) {
  return (
    <section className="rounded border border-slate-200 dark:border-slate-800 p-4">
      <h2 className="font-medium">{title}</h2>
      <p className="mt-1 min-h-10 text-sm text-slate-600 dark:text-slate-400">{description}</p>
      <Button className="mt-3" disabled={disabled} onClick={onConnect}>
        {busy ? "Connecting…" : `Connect ${title}`}
      </Button>
    </section>
  );
}

function Tool({ name, detail }: { name: string; detail: string }) {
  return (
    <li className="rounded bg-slate-50 dark:bg-slate-900 px-3 py-2">
      <code className="text-slate-900 dark:text-slate-100">{name}</code>
      <div className="mt-0.5 text-xs">{detail}</div>
    </li>
  );
}
