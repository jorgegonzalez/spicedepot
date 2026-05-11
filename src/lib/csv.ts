// Minimal CSV import/export for relationships.
//
// Format (no quoting needed — SpiceDB object IDs can't contain commas):
//
//   resource_type,resource_id,relation,subject_type,subject_id,subject_relation
//   document,doc1,viewer,user,alice,
//   document,doc2,viewer,user,alice,
//   group,eng,member,user,alice,member
//
// Rules:
//   - First row may be a header (we detect it by the literal string
//     `resource_type` in column 1 and skip).
//   - Lines starting with `#` are treated as comments.
//   - Blank lines are skipped.
//   - The trailing `subject_relation` column is optional; rows with five
//     columns instead of six are accepted (empty subject_relation).
//   - Caveats / caveat context are out of scope for v1 — see WriteRelationships
//     proto if you need to extend this.

import type { RelationshipInput, RelationshipRow } from "./api";

const HEADER = [
  "resource_type",
  "resource_id",
  "relation",
  "subject_type",
  "subject_id",
  "subject_relation",
] as const;

export interface ParseResult {
  rows: RelationshipInput[];
  /** Line numbers (1-based, including the header) that were rejected. */
  errors: { line: number; reason: string; raw: string }[];
}

export function parseCsv(text: string): ParseResult {
  const rows: RelationshipInput[] = [];
  const errors: ParseResult["errors"] = [];
  const lines = text.replace(/\r\n?/g, "\n").split("\n");

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trim();
    const lineNo = i + 1;
    if (!trimmed) continue;
    if (trimmed.startsWith("#")) continue;

    const cols = trimmed.split(",").map((c) => c.trim());

    // Skip the header row if present.
    if (i === 0 && cols[0] === HEADER[0]) continue;

    if (cols.length !== 5 && cols.length !== 6) {
      errors.push({
        line: lineNo,
        reason: `expected 5 or 6 columns, got ${cols.length}`,
        raw,
      });
      continue;
    }
    const [
      resource_type,
      resource_id,
      relation,
      subject_type,
      subject_id,
      subject_relation = "",
    ] = cols;

    if (
      !resource_type ||
      !resource_id ||
      !relation ||
      !subject_type ||
      !subject_id
    ) {
      errors.push({
        line: lineNo,
        reason: "missing required field (only subject_relation is optional)",
        raw,
      });
      continue;
    }

    rows.push({
      resource_type,
      resource_id,
      relation,
      subject_type,
      subject_id,
      subject_relation: subject_relation || undefined,
    });
  }

  return { rows, errors };
}

export function formatCsv(rows: RelationshipRow[]): string {
  const out: string[] = [HEADER.join(",")];
  for (const r of rows) {
    out.push(
      [
        r.resource_type,
        r.resource_id,
        r.relation,
        r.subject_type,
        r.subject_id,
        r.subject_relation ?? "",
      ].join(","),
    );
  }
  return out.join("\n") + "\n";
}

/** Trigger a browser download of `text` as `filename`. */
export function downloadText(filename: string, text: string, mime = "text/csv") {
  const blob = new Blob([text], { type: `${mime};charset=utf-8` });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  // Defer revoke so Safari/WebKit has time to grab the blob.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
