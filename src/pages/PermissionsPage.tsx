import { FormEvent, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api, CheckPermissionInput, CheckPermissionOutput } from "@/lib/api";
import { useActiveConnection } from "@/lib/store";
import { Banner, Button, Field, Input } from "@/components/ui";

const emptyInput: CheckPermissionInput = {
  resource_type: "",
  resource_id: "",
  permission: "",
  subject_type: "",
  subject_id: "",
  subject_relation: "",
};

export function PermissionsPage() {
  const { activeConnectionId } = useActiveConnection();
  const [input, setInput] = useState<CheckPermissionInput>(emptyInput);

  const checkMut = useMutation({
    mutationFn: () =>
      api.checkPermission(activeConnectionId!, {
        ...input,
        subject_relation: input.subject_relation || undefined,
      }),
  });

  if (!activeConnectionId) {
    return (
      <div className="p-6 text-sm text-slate-400">
        Select a connection from the header to run permission checks.
      </div>
    );
  }

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    checkMut.mutate();
  };

  const required = (k: keyof CheckPermissionInput) =>
    k !== "subject_relation";

  return (
    <div className="grid h-full grid-cols-[420px_1fr] gap-0 overflow-hidden">
      <section className="flex min-h-0 flex-col overflow-hidden border-r border-slate-800">
        <div className="border-b border-slate-800 px-4 py-3">
          <h1 className="text-sm font-medium uppercase tracking-wide text-slate-400">
            Check permission
          </h1>
          <p className="mt-1 text-xs text-slate-500">
            Calls{" "}
            <span className="font-mono text-slate-400">
              PermissionsService.CheckPermission
            </span>
            . Consistency: fully consistent.
          </p>
        </div>

        <form
          onSubmit={onSubmit}
          className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto p-4"
        >
          <fieldset className="space-y-3 rounded border border-slate-800 p-3">
            <legend className="px-1 text-xs uppercase tracking-wide text-slate-500">
              Resource
            </legend>
            <Field label="Type">
              <Input
                required={required("resource_type")}
                value={input.resource_type}
                onChange={(e) =>
                  setInput((p) => ({ ...p, resource_type: e.target.value }))
                }
                placeholder="document"
              />
            </Field>
            <Field label="ID">
              <Input
                required={required("resource_id")}
                value={input.resource_id}
                onChange={(e) =>
                  setInput((p) => ({ ...p, resource_id: e.target.value }))
                }
                placeholder="doc1"
              />
            </Field>
          </fieldset>

          <Field label="Permission">
            <Input
              required
              value={input.permission}
              onChange={(e) =>
                setInput((p) => ({ ...p, permission: e.target.value }))
              }
              placeholder="view"
            />
          </Field>

          <fieldset className="space-y-3 rounded border border-slate-800 p-3">
            <legend className="px-1 text-xs uppercase tracking-wide text-slate-500">
              Subject
            </legend>
            <Field label="Type">
              <Input
                required={required("subject_type")}
                value={input.subject_type}
                onChange={(e) =>
                  setInput((p) => ({ ...p, subject_type: e.target.value }))
                }
                placeholder="user"
              />
            </Field>
            <Field label="ID">
              <Input
                required={required("subject_id")}
                value={input.subject_id}
                onChange={(e) =>
                  setInput((p) => ({ ...p, subject_id: e.target.value }))
                }
                placeholder="alice"
              />
            </Field>
            <Field
              label="Relation (optional)"
              hint="For subject-relations like group:eng#member"
            >
              <Input
                value={input.subject_relation ?? ""}
                onChange={(e) =>
                  setInput((p) => ({ ...p, subject_relation: e.target.value }))
                }
                placeholder="member"
              />
            </Field>
          </fieldset>

          <div className="mt-2 flex gap-2">
            <Button type="submit" disabled={checkMut.isPending}>
              {checkMut.isPending ? "Checking…" : "Check"}
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={() => {
                setInput(emptyInput);
                checkMut.reset();
              }}
            >
              Clear
            </Button>
          </div>
        </form>
      </section>

      <section className="flex min-h-0 flex-col overflow-hidden">
        <div className="border-b border-slate-800 px-4 py-3">
          <h2 className="text-sm font-medium uppercase tracking-wide text-slate-400">
            Result
          </h2>
        </div>
        <div className="min-h-0 flex-1 overflow-auto p-4">
          {checkMut.error && (
            <Banner tone="error">
              {(checkMut.error as Error).message}
            </Banner>
          )}
          {checkMut.data && <ResultPanel data={checkMut.data} input={input} />}
          {!checkMut.data && !checkMut.error && (
            <div className="text-sm text-slate-500">
              Fill in the form and click Check. The result will appear here.
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

function ResultPanel({
  data,
  input,
}: {
  data: CheckPermissionOutput;
  input: CheckPermissionInput;
}) {
  const tone = {
    has_permission: {
      bg: "bg-emerald-950/60 border-emerald-700",
      text: "text-emerald-200",
      label: "Allowed",
    },
    no_permission: {
      bg: "bg-red-950/40 border-red-800",
      text: "text-red-200",
      label: "Denied",
    },
    conditional_permission: {
      bg: "bg-amber-950/40 border-amber-700",
      text: "text-amber-200",
      label: "Conditional",
    },
    unspecified: {
      bg: "bg-slate-900 border-slate-700",
      text: "text-slate-300",
      label: "Unspecified",
    },
  }[data.permissionship];

  const subjectSuffix = input.subject_relation
    ? `#${input.subject_relation}`
    : "";

  return (
    <div className="space-y-3">
      <div
        className={`rounded border px-4 py-3 ${tone.bg}`}
        role="status"
        aria-live="polite"
      >
        <div className={`text-lg font-semibold ${tone.text}`}>{tone.label}</div>
        <div className="mt-1 font-mono text-xs text-slate-400">
          {input.resource_type}:{input.resource_id}#{input.permission} @{" "}
          {input.subject_type}:{input.subject_id}
          {subjectSuffix}
        </div>
      </div>

      {data.permissionship === "conditional_permission" && (
        <div className="rounded border border-amber-800/60 bg-amber-950/30 p-3 text-sm text-amber-100">
          <div className="mb-1 font-medium">Caveat needs context</div>
          <div className="text-xs text-amber-200/70">
            Missing fields:{" "}
            {data.missing_context.length > 0 ? (
              <span className="font-mono">
                {data.missing_context.join(", ")}
              </span>
            ) : (
              <span className="italic">none reported</span>
            )}
          </div>
        </div>
      )}

      {data.checked_at && (
        <div className="text-xs text-slate-500">
          <span className="uppercase tracking-wide">checked_at:</span>{" "}
          <span className="font-mono">{data.checked_at}</span>
        </div>
      )}
    </div>
  );
}
