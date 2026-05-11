import { FormEvent, useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import {
  api,
  LookupResourceItem,
  LookupResourcesInput,
  LookupSubjectItem,
  LookupSubjectsInput,
  Permissionship,
} from "@/lib/api";
import { useActiveConnection } from "@/lib/store";
import { Banner, Button, Field, Input } from "@/components/ui";

type Mode = "resources" | "subjects";

const emptyResources: LookupResourcesInput = {
  resource_type: "",
  permission: "",
  subject_type: "",
  subject_id: "",
  subject_relation: "",
  limit: 100,
};

const emptySubjects: LookupSubjectsInput = {
  resource_type: "",
  resource_id: "",
  permission: "",
  subject_type: "",
  subject_relation: "",
  limit: 100,
};

export function LookupPage() {
  const { activeConnectionId } = useActiveConnection();
  const [mode, setMode] = useState<Mode>("resources");
  const [resourcesInput, setResourcesInput] =
    useState<LookupResourcesInput>(emptyResources);
  const [subjectsInput, setSubjectsInput] =
    useState<LookupSubjectsInput>(emptySubjects);

  const resourcesMut = useMutation({
    mutationFn: () =>
      api.lookupResources(activeConnectionId!, {
        ...resourcesInput,
        subject_relation: resourcesInput.subject_relation || undefined,
        limit: resourcesInput.limit || undefined,
      }),
  });

  const subjectsMut = useMutation({
    mutationFn: () =>
      api.lookupSubjects(activeConnectionId!, {
        ...subjectsInput,
        subject_relation: subjectsInput.subject_relation || undefined,
        limit: subjectsInput.limit || undefined,
      }),
  });

  if (!activeConnectionId) {
    return (
      <div className="p-6 text-sm text-slate-400">
        Select a connection from the header to run lookups.
      </div>
    );
  }

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (mode === "resources") resourcesMut.mutate();
    else subjectsMut.mutate();
  };

  const onClear = () => {
    if (mode === "resources") {
      setResourcesInput(emptyResources);
      resourcesMut.reset();
    } else {
      setSubjectsInput(emptySubjects);
      subjectsMut.reset();
    }
  };

  const currentMut = mode === "resources" ? resourcesMut : subjectsMut;

  return (
    <div className="grid h-full grid-cols-[420px_1fr] gap-0 overflow-hidden">
      <section className="flex min-h-0 flex-col overflow-hidden border-r border-slate-800">
        <div className="border-b border-slate-800 px-4 py-3">
          <h1 className="text-sm font-medium uppercase tracking-wide text-slate-400">
            Lookup
          </h1>
          <div className="mt-2 inline-flex rounded border border-slate-700 p-0.5 text-xs">
            <button
              type="button"
              onClick={() => setMode("resources")}
              className={`rounded px-3 py-1 transition ${
                mode === "resources"
                  ? "bg-slate-800 text-slate-100"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              Find resources
            </button>
            <button
              type="button"
              onClick={() => setMode("subjects")}
              className={`rounded px-3 py-1 transition ${
                mode === "subjects"
                  ? "bg-slate-800 text-slate-100"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              Find subjects
            </button>
          </div>
          <p className="mt-2 text-xs text-slate-500">
            {mode === "resources"
              ? "Given a subject + permission, list the resources they can access."
              : "Given a resource + permission, list the subjects that can access it."}
          </p>
        </div>

        <form
          onSubmit={onSubmit}
          className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto p-4"
        >
          {mode === "resources" ? (
            <ResourcesForm
              input={resourcesInput}
              setInput={setResourcesInput}
            />
          ) : (
            <SubjectsForm input={subjectsInput} setInput={setSubjectsInput} />
          )}

          <div className="mt-2 flex gap-2">
            <Button type="submit" disabled={currentMut.isPending}>
              {currentMut.isPending ? "Looking up…" : "Look up"}
            </Button>
            <Button type="button" variant="ghost" onClick={onClear}>
              Clear
            </Button>
          </div>
        </form>
      </section>

      <section className="flex min-h-0 flex-col overflow-hidden">
        <ResultsHeader mode={mode} mut={currentMut} />
        <div className="min-h-0 flex-1 overflow-auto">
          {currentMut.error && (
            <div className="p-4">
              <Banner tone="error">
                {(currentMut.error as Error).message}
              </Banner>
            </div>
          )}
          {!currentMut.data && !currentMut.error && (
            <div className="p-4 text-sm text-slate-500">
              Fill in the form and click Look up.
            </div>
          )}
          {currentMut.data &&
            (mode === "resources" ? (
              <ResourcesTable items={resourcesMut.data?.items ?? []} />
            ) : (
              <SubjectsTable items={subjectsMut.data?.items ?? []} />
            ))}
        </div>
      </section>
    </div>
  );
}

function ResultsHeader({
  mode,
  mut,
}: {
  mode: Mode;
  mut: { data?: { items: unknown[]; looked_up_at: string | null } };
}) {
  const count = mut.data?.items.length ?? 0;
  return (
    <div className="border-b border-slate-800 px-4 py-3">
      <h2 className="text-sm font-medium uppercase tracking-wide text-slate-400">
        Results
      </h2>
      {mut.data && (
        <p className="mt-1 text-xs text-slate-500">
          {count === 0
            ? `No matching ${mode}`
            : `${count} ${count === 1 ? "result" : "results"}`}
          {mut.data.looked_up_at && (
            <>
              {" · "}
              <span className="font-mono">{mut.data.looked_up_at}</span>
            </>
          )}
        </p>
      )}
    </div>
  );
}

function ResourcesForm({
  input,
  setInput,
}: {
  input: LookupResourcesInput;
  setInput: React.Dispatch<React.SetStateAction<LookupResourcesInput>>;
}) {
  return (
    <>
      <Field label="Resource type" hint="Type to enumerate (e.g. document)">
        <Input
          required
          value={input.resource_type}
          onChange={(e) =>
            setInput((p) => ({ ...p, resource_type: e.target.value }))
          }
          placeholder="document"
        />
      </Field>
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
            required
            value={input.subject_type}
            onChange={(e) =>
              setInput((p) => ({ ...p, subject_type: e.target.value }))
            }
            placeholder="user"
          />
        </Field>
        <Field label="ID">
          <Input
            required
            value={input.subject_id}
            onChange={(e) =>
              setInput((p) => ({ ...p, subject_id: e.target.value }))
            }
            placeholder="alice"
          />
        </Field>
        <Field label="Relation (optional)">
          <Input
            value={input.subject_relation ?? ""}
            onChange={(e) =>
              setInput((p) => ({ ...p, subject_relation: e.target.value }))
            }
            placeholder="member"
          />
        </Field>
      </fieldset>
      <Field
        label="Limit"
        hint="SpiceDB caps server-side (typically 1000). 0 = server default."
      >
        <Input
          type="number"
          min={0}
          max={1000}
          value={input.limit ?? 0}
          onChange={(e) =>
            setInput((p) => ({
              ...p,
              limit: Number.isFinite(+e.target.value) ? +e.target.value : 0,
            }))
          }
        />
      </Field>
    </>
  );
}

function SubjectsForm({
  input,
  setInput,
}: {
  input: LookupSubjectsInput;
  setInput: React.Dispatch<React.SetStateAction<LookupSubjectsInput>>;
}) {
  return (
    <>
      <fieldset className="space-y-3 rounded border border-slate-800 p-3">
        <legend className="px-1 text-xs uppercase tracking-wide text-slate-500">
          Resource
        </legend>
        <Field label="Type">
          <Input
            required
            value={input.resource_type}
            onChange={(e) =>
              setInput((p) => ({ ...p, resource_type: e.target.value }))
            }
            placeholder="document"
          />
        </Field>
        <Field label="ID">
          <Input
            required
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
      <Field
        label="Subject type"
        hint="Type to enumerate (e.g. user)"
      >
        <Input
          required
          value={input.subject_type}
          onChange={(e) =>
            setInput((p) => ({ ...p, subject_type: e.target.value }))
          }
          placeholder="user"
        />
      </Field>
      <Field label="Subject relation (optional)">
        <Input
          value={input.subject_relation ?? ""}
          onChange={(e) =>
            setInput((p) => ({ ...p, subject_relation: e.target.value }))
          }
          placeholder="member"
        />
      </Field>
      <Field
        label="Limit"
        hint="Cap on results. Note: as of SpiceDB 1.40+, limit is not enforced for LookupSubjects."
      >
        <Input
          type="number"
          min={0}
          max={1000}
          value={input.limit ?? 0}
          onChange={(e) =>
            setInput((p) => ({
              ...p,
              limit: Number.isFinite(+e.target.value) ? +e.target.value : 0,
            }))
          }
        />
      </Field>
    </>
  );
}

function PermissionshipBadge({ p }: { p: Permissionship }) {
  const tone = {
    has_permission: "bg-emerald-950/60 border-emerald-700 text-emerald-200",
    no_permission: "bg-red-950/40 border-red-800 text-red-200",
    conditional_permission: "bg-amber-950/40 border-amber-700 text-amber-200",
    unspecified: "bg-slate-900 border-slate-700 text-slate-300",
  }[p];
  const label = {
    has_permission: "Allowed",
    no_permission: "Denied",
    conditional_permission: "Conditional",
    unspecified: "?",
  }[p];
  return (
    <span
      className={`inline-block rounded border px-1.5 py-0.5 text-[10px] uppercase tracking-wide ${tone}`}
    >
      {label}
    </span>
  );
}

function ResourcesTable({ items }: { items: LookupResourceItem[] }) {
  const sorted = useMemo(
    () => [...items].sort((a, b) => a.resource_id.localeCompare(b.resource_id)),
    [items],
  );
  if (sorted.length === 0) {
    return (
      <div className="p-4 text-sm text-slate-500">
        No resources match this subject + permission.
      </div>
    );
  }
  return (
    <table className="w-full text-sm">
      <thead className="sticky top-0 bg-slate-950">
        <tr className="border-b border-slate-800 text-left text-xs uppercase tracking-wide text-slate-500">
          <th className="px-4 py-2 font-medium">Resource ID</th>
          <th className="px-4 py-2 font-medium">Permission</th>
          <th className="px-4 py-2 font-medium">Missing context</th>
        </tr>
      </thead>
      <tbody>
        {sorted.map((it) => (
          <tr
            key={it.resource_id}
            className="border-b border-slate-900 font-mono"
          >
            <td className="px-4 py-1.5">{it.resource_id}</td>
            <td className="px-4 py-1.5">
              <PermissionshipBadge p={it.permissionship} />
            </td>
            <td className="px-4 py-1.5 text-xs text-slate-500">
              {it.missing_context.length === 0
                ? "—"
                : it.missing_context.join(", ")}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function SubjectsTable({ items }: { items: LookupSubjectItem[] }) {
  const sorted = useMemo(
    () => [...items].sort((a, b) => a.subject_id.localeCompare(b.subject_id)),
    [items],
  );
  if (sorted.length === 0) {
    return (
      <div className="p-4 text-sm text-slate-500">
        No subjects match this resource + permission.
      </div>
    );
  }
  return (
    <table className="w-full text-sm">
      <thead className="sticky top-0 bg-slate-950">
        <tr className="border-b border-slate-800 text-left text-xs uppercase tracking-wide text-slate-500">
          <th className="px-4 py-2 font-medium">Subject ID</th>
          <th className="px-4 py-2 font-medium">Permission</th>
          <th className="px-4 py-2 font-medium">Excluded / context</th>
        </tr>
      </thead>
      <tbody>
        {sorted.map((it) => (
          <tr
            key={it.subject_id}
            className="border-b border-slate-900 font-mono"
          >
            <td className="px-4 py-1.5">
              {it.subject_id === "*" ? (
                <span className="text-amber-300">* (wildcard)</span>
              ) : (
                it.subject_id
              )}
            </td>
            <td className="px-4 py-1.5">
              <PermissionshipBadge p={it.permissionship} />
            </td>
            <td className="px-4 py-1.5 text-xs text-slate-500">
              {it.excluded_subject_ids.length > 0 &&
                `excluded: ${it.excluded_subject_ids.join(", ")}`}
              {it.excluded_subject_ids.length > 0 &&
                it.missing_context.length > 0 &&
                " · "}
              {it.missing_context.length > 0 &&
                `missing: ${it.missing_context.join(", ")}`}
              {it.excluded_subject_ids.length === 0 &&
                it.missing_context.length === 0 &&
                "—"}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
