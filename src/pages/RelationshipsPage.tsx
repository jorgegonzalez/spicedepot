import { FormEvent, useMemo, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  api,
  BulkDeleteOutput,
  BulkWriteOutput,
  ReadRelationshipsOutput,
  RelationshipFilterInput,
  RelationshipInput,
  RelationshipRow,
  WriteOperation,
} from "@/lib/api";
import { useActiveConnection } from "@/lib/store";
import { Banner, Button, Field, Input } from "@/components/ui";
import { downloadText, formatCsv, parseCsv } from "@/lib/csv";

const emptyFilter: RelationshipFilterInput = {
  resource_type: "",
  resource_id: "",
  relation: "",
  subject_type: "",
  subject_id: "",
  subject_relation: "",
};

const emptyRelationship: RelationshipInput = {
  resource_type: "",
  resource_id: "",
  relation: "",
  subject_type: "",
  subject_id: "",
  subject_relation: "",
};

export function RelationshipsPage() {
  const { activeConnectionId } = useActiveConnection();
  const qc = useQueryClient();
  const [filter, setFilter] = useState<RelationshipFilterInput>(emptyFilter);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newRel, setNewRel] = useState<RelationshipInput>(emptyRelationship);
  const [newOp, setNewOp] = useState<"create" | "touch">("touch");
  const [bannerError, setBannerError] = useState<string | null>(null);

  const readMut = useMutation<ReadRelationshipsOutput, Error>({
    mutationFn: () =>
      api.readRelationships(activeConnectionId!, normalizeFilter(filter)),
  });

  const writeMut = useMutation<
    void,
    Error,
    { op: WriteOperation; rel: RelationshipInput }
  >({
    mutationFn: ({ op, rel }) =>
      api.writeRelationship(activeConnectionId!, op, normalizeRelationship(rel)),
    onMutate: () => setBannerError(null),
    onError: (e) => setBannerError(e.message),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["relationships"] });
      // Re-run the current read so the table reflects the change.
      if (readMut.data || readMut.isPending) readMut.mutate();
    },
  });

  const [bulkResult, setBulkResult] = useState<BulkDeleteOutput | null>(null);
  const [importResult, setImportResult] = useState<{
    out: BulkWriteOutput;
    rejected: number;
  } | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const bulkDeleteMut = useMutation<BulkDeleteOutput, Error>({
    mutationFn: () =>
      api.bulkDeleteRelationships(activeConnectionId!, normalizeFilter(filter)),
    onMutate: () => {
      setBannerError(null);
      setBulkResult(null);
    },
    onError: (e) => setBannerError(e.message),
    onSuccess: (out) => {
      setBulkResult(out);
      if (readMut.data || readMut.isPending) readMut.mutate();
    },
  });

  const importMut = useMutation<
    BulkWriteOutput,
    Error,
    { rels: RelationshipInput[]; rejected: number }
  >({
    mutationFn: ({ rels }) =>
      api.bulkWriteRelationships(activeConnectionId!, "touch", rels),
    onMutate: () => {
      setBannerError(null);
      setImportResult(null);
    },
    onError: (e) => setBannerError(e.message),
    onSuccess: (out, vars) => {
      setImportResult({ out, rejected: vars.rejected });
      if (readMut.data || readMut.isPending) readMut.mutate();
    },
  });

  const onExportCsv = () => {
    const rows = readMut.data?.items ?? [];
    if (rows.length === 0) {
      setBannerError("Read some relationships first — nothing to export.");
      return;
    }
    const csv = formatCsv(rows);
    const stamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
    downloadText(`spicedepot-relationships-${stamp}.csv`, csv);
  };

  const onImportFileChosen = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = ""; // allow re-picking the same file later
    if (!file) return;
    const text = await file.text();
    const { rows, errors } = parseCsv(text);
    if (rows.length === 0) {
      setBannerError(
        errors.length > 0
          ? `Parsed 0 rows — ${errors.length} rejected. First error: line ${errors[0].line}: ${errors[0].reason}`
          : "Parsed 0 rows from the file.",
      );
      return;
    }
    if (
      !confirm(
        `Touch ${rows.length} relationship${rows.length === 1 ? "" : "s"} on this connection?` +
          (errors.length > 0
            ? `\n\n${errors.length} row${errors.length === 1 ? "" : "s"} will be skipped (parse errors).`
            : "") +
          `\n\nTOUCH is idempotent — existing rows won't error.`,
      )
    ) {
      return;
    }
    importMut.mutate({ rels: rows, rejected: errors.length });
  };

  const onBulkDelete = () => {
    if (!filter.resource_type) {
      setBannerError("Set a resource type before deleting.");
      return;
    }
    const summary = filterSummary(filter);
    if (
      !confirm(
        `Delete every relationship matching:\n\n  ${summary}\n\nThis cannot be undone. Continue?`,
      )
    ) {
      return;
    }
    bulkDeleteMut.mutate();
  };

  if (!activeConnectionId) {
    return (
      <div className="p-6 text-sm text-slate-600 dark:text-slate-400">
        Select a connection from the header to browse relationships.
      </div>
    );
  }

  const onReadSubmit = (e: FormEvent) => {
    e.preventDefault();
    readMut.mutate();
  };

  const onAddSubmit = (e: FormEvent) => {
    e.preventDefault();
    writeMut.mutate({ op: newOp, rel: newRel });
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden">
      <div className="border-b border-slate-200 dark:border-slate-800 px-4 py-3">
        <div className="flex items-center justify-between">
          <h1 className="text-sm font-medium uppercase tracking-wide text-slate-600 dark:text-slate-400">
            Relationships
          </h1>
          <div className="flex gap-2">
            <input
              ref={fileInputRef}
              type="file"
              accept=".csv,text/csv,text/plain"
              onChange={onImportFileChosen}
              className="hidden"
            />
            <Button
              variant="ghost"
              onClick={() => fileInputRef.current?.click()}
              disabled={importMut.isPending}
              title="Import a CSV of relationships (TOUCH semantics)"
            >
              {importMut.isPending ? "Importing…" : "Import CSV"}
            </Button>
            <Button
              variant="ghost"
              onClick={onExportCsv}
              disabled={!readMut.data || readMut.data.items.length === 0}
              title="Download the current read result as CSV"
            >
              Export CSV
            </Button>
            <Button
              variant={showAddForm ? "ghost" : "secondary"}
              onClick={() => {
                setShowAddForm((v) => !v);
                if (showAddForm) setNewRel(emptyRelationship);
              }}
            >
              {showAddForm ? "Cancel add" : "+ Add relationship"}
            </Button>
          </div>
        </div>
      </div>

      {showAddForm && (
        <form
          onSubmit={onAddSubmit}
          className="grid gap-3 border-b border-slate-200 dark:border-slate-800 bg-slate-100/40 dark:bg-slate-900/40 p-4 lg:grid-cols-[1fr_1fr_1fr_auto]"
        >
          <ObjectFields
            label="Resource"
            type={newRel.resource_type}
            id={newRel.resource_id}
            onTypeChange={(v) =>
              setNewRel((p) => ({ ...p, resource_type: v }))
            }
            onIdChange={(v) => setNewRel((p) => ({ ...p, resource_id: v }))}
            required
          />
          <Field label="Relation">
            <Input
              required
              value={newRel.relation}
              onChange={(e) =>
                setNewRel((p) => ({ ...p, relation: e.target.value }))
              }
              placeholder="viewer"
            />
          </Field>
          <SubjectFields
            type={newRel.subject_type}
            id={newRel.subject_id}
            rel={newRel.subject_relation ?? ""}
            onTypeChange={(v) =>
              setNewRel((p) => ({ ...p, subject_type: v }))
            }
            onIdChange={(v) => setNewRel((p) => ({ ...p, subject_id: v }))}
            onRelChange={(v) =>
              setNewRel((p) => ({ ...p, subject_relation: v }))
            }
            required
          />
          <div className="flex flex-col gap-2 self-end">
            <div className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400">
              <label className="flex items-center gap-1">
                <input
                  type="radio"
                  name="op"
                  checked={newOp === "touch"}
                  onChange={() => setNewOp("touch")}
                />
                Touch
              </label>
              <label className="flex items-center gap-1">
                <input
                  type="radio"
                  name="op"
                  checked={newOp === "create"}
                  onChange={() => setNewOp("create")}
                />
                Create
              </label>
            </div>
            <Button type="submit" disabled={writeMut.isPending}>
              {writeMut.isPending ? "Writing…" : "Write"}
            </Button>
          </div>
        </form>
      )}

      <form
        onSubmit={onReadSubmit}
        className="grid gap-3 border-b border-slate-200 dark:border-slate-800 p-4 lg:grid-cols-[1fr_1fr_1fr_auto]"
      >
        <ObjectFields
          label="Resource filter"
          type={filter.resource_type}
          id={filter.resource_id ?? ""}
          onTypeChange={(v) =>
            setFilter((p) => ({ ...p, resource_type: v }))
          }
          onIdChange={(v) => setFilter((p) => ({ ...p, resource_id: v }))}
          required
        />
        <Field label="Relation (optional)">
          <Input
            value={filter.relation ?? ""}
            onChange={(e) =>
              setFilter((p) => ({ ...p, relation: e.target.value }))
            }
            placeholder="any"
          />
        </Field>
        <SubjectFields
          type={filter.subject_type ?? ""}
          id={filter.subject_id ?? ""}
          rel={filter.subject_relation ?? ""}
          onTypeChange={(v) =>
            setFilter((p) => ({ ...p, subject_type: v }))
          }
          onIdChange={(v) => setFilter((p) => ({ ...p, subject_id: v }))}
          onRelChange={(v) =>
            setFilter((p) => ({ ...p, subject_relation: v }))
          }
        />
        <div className="flex flex-col gap-2 self-end">
          <Button type="submit" disabled={readMut.isPending}>
            {readMut.isPending ? "Reading…" : "Read"}
          </Button>
          <Button
            type="button"
            variant="danger"
            onClick={onBulkDelete}
            disabled={bulkDeleteMut.isPending}
            title="Delete every relationship matching the current filter. SpiceDB caps server-side (default 1000)."
          >
            {bulkDeleteMut.isPending ? "Deleting…" : "Delete all matching"}
          </Button>
        </div>
      </form>

      {bulkResult && (
        <div className="p-3">
          <Banner tone={bulkResult.progress === "complete" ? "success" : "info"}>
            Deleted {bulkResult.deleted_count}{" "}
            {bulkResult.deleted_count === 1 ? "relationship" : "relationships"}
            {bulkResult.progress === "partial" &&
              " (server hit limit — re-run to continue)"}
            {bulkResult.deleted_at && (
              <span className="ml-2 font-mono text-xs opacity-70">
                {bulkResult.deleted_at}
              </span>
            )}
          </Banner>
        </div>
      )}

      {importResult && (
        <div className="p-3">
          <Banner tone="success">
            Imported {importResult.out.written_count}{" "}
            {importResult.out.written_count === 1
              ? "relationship"
              : "relationships"}{" "}
            in {importResult.out.chunks}{" "}
            {importResult.out.chunks === 1 ? "batch" : "batches"}
            {importResult.rejected > 0 &&
              ` · ${importResult.rejected} row${importResult.rejected === 1 ? "" : "s"} skipped (parse errors)`}
          </Banner>
        </div>
      )}

      {bannerError && (
        <div className="p-3">
          <Banner tone="error">{bannerError}</Banner>
        </div>
      )}
      {readMut.error && (
        <div className="p-3">
          <Banner tone="error">{readMut.error.message}</Banner>
        </div>
      )}

      <ResultsHeader data={readMut.data} />

      <div className="min-h-0 flex-1 overflow-auto">
        {!readMut.data && !readMut.error && (
          <div className="p-4 text-sm text-slate-500">
            Set a resource type filter and click Read.
          </div>
        )}
        {readMut.data && (
          <RelationshipTable
            rows={readMut.data.items}
            onDelete={(row) =>
              writeMut.mutate({ op: "delete", rel: rowToInput(row) })
            }
            deletingKey={
              writeMut.isPending && writeMut.variables?.op === "delete"
                ? rowKey(writeMut.variables.rel)
                : null
            }
          />
        )}
      </div>
    </div>
  );
}

function ResultsHeader({ data }: { data: ReadRelationshipsOutput | undefined }) {
  if (!data) return null;
  const n = data.items.length;
  return (
    <div className="border-b border-slate-200 dark:border-slate-800 px-4 py-2 text-xs text-slate-500">
      {n} {n === 1 ? "relationship" : "relationships"}
      {data.read_at && (
        <>
          {" · "}
          <span className="font-mono">{data.read_at}</span>
        </>
      )}
    </div>
  );
}

function RelationshipTable({
  rows,
  onDelete,
  deletingKey,
}: {
  rows: RelationshipRow[];
  onDelete: (row: RelationshipRow) => void;
  deletingKey: string | null;
}) {
  const sorted = useMemo(
    () =>
      [...rows].sort((a, b) => {
        const k = (r: RelationshipRow) =>
          `${r.resource_type}:${r.resource_id}#${r.relation}@${r.subject_type}:${r.subject_id}`;
        return k(a).localeCompare(k(b));
      }),
    [rows],
  );
  if (sorted.length === 0) {
    return (
      <div className="p-4 text-sm text-slate-500">
        No relationships match this filter.
      </div>
    );
  }
  return (
    <table className="w-full text-sm">
      <thead className="sticky top-0 bg-white dark:bg-slate-950">
        <tr className="border-b border-slate-200 dark:border-slate-800 text-left text-xs uppercase tracking-wide text-slate-500">
          <th className="px-4 py-2 font-medium">Resource</th>
          <th className="px-4 py-2 font-medium">Relation</th>
          <th className="px-4 py-2 font-medium">Subject</th>
          <th className="px-4 py-2 font-medium">Caveat</th>
          <th className="px-4 py-2"></th>
        </tr>
      </thead>
      <tbody>
        {sorted.map((row) => {
          const key = rowKey(rowToInput(row));
          const isDeleting = deletingKey === key;
          return (
            <tr key={key} className="border-b border-slate-200 dark:border-slate-900 font-mono">
              <td className="px-4 py-1.5">
                {row.resource_type}:{row.resource_id}
              </td>
              <td className="px-4 py-1.5">{row.relation}</td>
              <td className="px-4 py-1.5">
                {row.subject_type}:{row.subject_id}
                {row.subject_relation && `#${row.subject_relation}`}
              </td>
              <td className="px-4 py-1.5 text-xs text-slate-500">
                {row.caveat_name ?? "—"}
              </td>
              <td className="px-4 py-1.5 text-right">
                <Button
                  variant="ghost"
                  onClick={() => {
                    if (
                      confirm(
                        `Delete ${row.resource_type}:${row.resource_id}#${row.relation}@${row.subject_type}:${row.subject_id}?`,
                      )
                    ) {
                      onDelete(row);
                    }
                  }}
                  disabled={isDeleting}
                >
                  {isDeleting ? "…" : "×"}
                </Button>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function ObjectFields({
  label,
  type,
  id,
  onTypeChange,
  onIdChange,
  required,
}: {
  label: string;
  type: string;
  id: string;
  onTypeChange: (v: string) => void;
  onIdChange: (v: string) => void;
  required?: boolean;
}) {
  return (
    <fieldset className="rounded border border-slate-200 dark:border-slate-800 p-2">
      <legend className="px-1 text-[10px] uppercase tracking-wide text-slate-500">
        {label}
      </legend>
      <div className="grid grid-cols-2 gap-2">
        <Input
          required={required}
          value={type}
          onChange={(e) => onTypeChange(e.target.value)}
          placeholder="type"
        />
        <Input
          required={required && !label.toLowerCase().includes("filter")}
          value={id}
          onChange={(e) => onIdChange(e.target.value)}
          placeholder="id"
        />
      </div>
    </fieldset>
  );
}

function SubjectFields({
  type,
  id,
  rel,
  onTypeChange,
  onIdChange,
  onRelChange,
  required,
}: {
  type: string;
  id: string;
  rel: string;
  onTypeChange: (v: string) => void;
  onIdChange: (v: string) => void;
  onRelChange: (v: string) => void;
  required?: boolean;
}) {
  return (
    <fieldset className="rounded border border-slate-200 dark:border-slate-800 p-2">
      <legend className="px-1 text-[10px] uppercase tracking-wide text-slate-500">
        Subject {required ? "" : "filter"}
      </legend>
      <div className="grid grid-cols-[1fr_1fr_1fr] gap-2">
        <Input
          required={required}
          value={type}
          onChange={(e) => onTypeChange(e.target.value)}
          placeholder="type"
        />
        <Input
          required={required}
          value={id}
          onChange={(e) => onIdChange(e.target.value)}
          placeholder="id"
        />
        <Input
          value={rel}
          onChange={(e) => onRelChange(e.target.value)}
          placeholder="#rel"
        />
      </div>
    </fieldset>
  );
}

function rowToInput(r: RelationshipRow): RelationshipInput {
  return {
    resource_type: r.resource_type,
    resource_id: r.resource_id,
    relation: r.relation,
    subject_type: r.subject_type,
    subject_id: r.subject_id,
    subject_relation: r.subject_relation ?? undefined,
  };
}

function rowKey(r: RelationshipInput): string {
  return `${r.resource_type}:${r.resource_id}#${r.relation}@${r.subject_type}:${r.subject_id}${
    r.subject_relation ? `#${r.subject_relation}` : ""
  }`;
}

function normalizeFilter(f: RelationshipFilterInput): RelationshipFilterInput {
  return {
    resource_type: f.resource_type,
    resource_id: emptyToUndef(f.resource_id),
    relation: emptyToUndef(f.relation),
    subject_type: emptyToUndef(f.subject_type),
    subject_id: emptyToUndef(f.subject_id),
    subject_relation: emptyToUndef(f.subject_relation),
  };
}

function normalizeRelationship(r: RelationshipInput): RelationshipInput {
  return {
    ...r,
    subject_relation: emptyToUndef(r.subject_relation),
  };
}

function emptyToUndef(s: string | undefined): string | undefined {
  return s && s.length > 0 ? s : undefined;
}

function filterSummary(f: RelationshipFilterInput): string {
  const parts: string[] = [`${f.resource_type}:${f.resource_id || "*"}`];
  if (f.relation) parts.push(`#${f.relation}`);
  if (f.subject_type) {
    parts.push(
      ` @ ${f.subject_type}:${f.subject_id || "*"}${
        f.subject_relation ? `#${f.subject_relation}` : ""
      }`,
    );
  }
  return parts.join("");
}
