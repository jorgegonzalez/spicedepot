import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { EditorState, Compartment } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import { oneDark } from "@codemirror/theme-one-dark";
import { api } from "@/lib/api";
import { useActiveConnection } from "@/lib/store";
import { spiceDbSchemaLanguage } from "@/lib/spicedbLang";
import { Banner, Button } from "@/components/ui";

export function SchemaPage() {
  const { activeConnectionId } = useActiveConnection();
  const qc = useQueryClient();
  const editorParent = useRef<HTMLDivElement | null>(null);
  const editorView = useRef<EditorView | null>(null);
  const editableComp = useRef(new Compartment());
  const [draft, setDraft] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savedAt, setSavedAt] = useState<string | null>(null);

  const schemaQuery = useQuery({
    queryKey: ["schema", activeConnectionId],
    queryFn: () => api.readSchema(activeConnectionId!),
    enabled: !!activeConnectionId,
  });

  const writeMut = useMutation({
    mutationFn: () => api.writeSchema(activeConnectionId!, draft),
    onMutate: () => setSaveError(null),
    onSuccess: () => {
      setDirty(false);
      setSavedAt(new Date().toISOString());
      qc.invalidateQueries({ queryKey: ["schema", activeConnectionId] });
    },
    onError: (err: Error) => setSaveError(err.message),
  });

  // Mount the editor once. We push fresh content via dispatch.
  useEffect(() => {
    if (!editorParent.current || editorView.current) return;
    const state = EditorState.create({
      doc: "",
      extensions: [
        lineNumbers(),
        history(),
        bracketMatching(),
        indentOnInput(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        oneDark,
        spiceDbSchemaLanguage(),
        EditorView.theme({
          "&": { backgroundColor: "transparent" },
          ".cm-gutters": { backgroundColor: "transparent" },
        }),
        editableComp.current.of(EditorView.editable.of(true)),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) {
            const text = u.state.doc.toString();
            setDraft(text);
            setDirty(true);
          }
        }),
      ],
    });
    editorView.current = new EditorView({
      state,
      parent: editorParent.current,
    });
    return () => {
      editorView.current?.destroy();
      editorView.current = null;
    };
  }, []);

  // Push remote schema into the editor whenever the query result changes.
  useEffect(() => {
    if (!editorView.current) return;
    const text = schemaQuery.data?.schema_text ?? "";
    if (text !== draft) {
      const view = editorView.current;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text },
      });
      setDraft(text);
      setDirty(false);
      setSavedAt(schemaQuery.data?.read_at ?? null);
    }
    // We intentionally don't depend on `draft` — only sync when the server doc changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [schemaQuery.data?.schema_text]);

  if (!activeConnectionId) {
    return (
      <div className="p-6 text-sm text-slate-400">
        Select a connection from the header (or create one in Connections) to
        view its schema.
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-slate-800 px-4 py-3">
        <div>
          <h1 className="text-sm font-medium uppercase tracking-wide text-slate-400">
            Schema
          </h1>
          <p className="text-xs text-slate-500">
            {schemaQuery.isFetching
              ? "Reading…"
              : savedAt
              ? `Last read ${new Date(savedAt).toLocaleString()}`
              : "—"}
            {dirty && " · unsaved changes"}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            onClick={() => schemaQuery.refetch()}
            disabled={schemaQuery.isFetching}
          >
            Refresh
          </Button>
          <Button
            onClick={() => writeMut.mutate()}
            disabled={!dirty || writeMut.isPending}
          >
            {writeMut.isPending ? "Writing…" : "Write schema"}
          </Button>
        </div>
      </div>

      {schemaQuery.error && (
        <div className="border-b border-slate-800 p-3">
          <Banner tone="error">
            Failed to read schema: {(schemaQuery.error as Error).message}
          </Banner>
        </div>
      )}
      {saveError && (
        <div className="border-b border-slate-800 p-3">
          <Banner tone="error">Write failed: {saveError}</Banner>
        </div>
      )}

      <div ref={editorParent} className="min-h-0 flex-1 overflow-hidden" />
    </div>
  );
}
