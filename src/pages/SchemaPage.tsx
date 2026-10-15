import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { EditorState, Compartment } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { bracketMatching, indentOnInput, defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { oneDark } from "@codemirror/theme-one-dark";
import { api } from "@/lib/api";
import { useActiveConnection, useSchemaDrafts, useTheme } from "@/lib/store";
import { spiceDbSchemaLanguage } from "@/lib/spicedbLang";
import { Banner, Button, Dialog } from "@/components/ui";

export function SchemaPage() {
  const { activeConnectionId } = useActiveConnection();
  return activeConnectionId ? <SchemaEditor key={activeConnectionId} connectionId={activeConnectionId} /> : (
    <div className="p-6 text-sm text-slate-600 dark:text-slate-400">Select a connection from the header (or create one in Connections) to view its schema.</div>
  );
}

function SchemaEditor({ connectionId }: { connectionId: string }) {
  const qc = useQueryClient();
  const { theme } = useTheme();
  const parent = useRef<HTMLDivElement>(null);
  const editor = useRef<EditorView | null>(null);
  const themeCompartment = useRef(new Compartment());
  const applyingRemote = useRef(false);
  const draft = useSchemaDrafts((s) => s.drafts[connectionId]);
  const dirty = draft !== undefined;
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const query = useQuery({ queryKey: ["schema", connectionId], queryFn: () => api.readSchema(connectionId) });
  const baseline = useRef<string | undefined>(undefined);

  const write = useMutation({
    mutationFn: (text: string) => api.writeSchema(connectionId, text),
    onSuccess: (_, submitted) => {
      baseline.current = submitted;
      const store = useSchemaDrafts.getState();
      // A completed write must not clear edits made while it was in flight.
      if (store.drafts[connectionId] === submitted) store.removeDraft(connectionId);
      qc.invalidateQueries({ queryKey: ["schema", connectionId] });
    },
  });

  useEffect(() => {
    const view = new EditorView({
      parent: parent.current!,
      state: EditorState.create({
        doc: useSchemaDrafts.getState().drafts[connectionId] ?? "",
        extensions: [
          lineNumbers(), history(), bracketMatching(), indentOnInput(),
          keymap.of([...defaultKeymap, ...historyKeymap]),
          themeCompartment.current.of([]), spiceDbSchemaLanguage(),
          EditorView.contentAttributes.of({ "aria-label": "Schema editor" }),
          EditorView.updateListener.of((update) => {
            if (!update.docChanged || applyingRemote.current) return;
            const text = update.state.doc.toString();
            const store = useSchemaDrafts.getState();
            if (text === baseline.current) store.removeDraft(connectionId);
            else store.saveDraft(connectionId, text);
          }),
        ],
      }),
    });
    editor.current = view;
    return () => { view.destroy(); editor.current = null; };
  }, [connectionId]);

  useEffect(() => {
    editor.current?.dispatch({ effects: themeCompartment.current.reconfigure(
      theme === "dark" ? oneDark : [syntaxHighlighting(defaultHighlightStyle), EditorView.theme({
        "&": { backgroundColor: "#fff", color: "#0f172a" },
        ".cm-gutters": { backgroundColor: "#f1f5f9", color: "#64748b" },
      })],
    ) });
  }, [theme]);

  useEffect(() => {
    if (!query.data || !editor.current) return;
    baseline.current = query.data.schema_text;
    if (useSchemaDrafts.getState().drafts[connectionId] !== undefined) return;
    applyingRemote.current = true;
    editor.current.dispatch({ changes: { from: 0, to: editor.current.state.doc.length, insert: query.data.schema_text } });
    applyingRemote.current = false;
  }, [query.data, connectionId]);

  const discard = () => {
    const view = editor.current;
    if (!view || !query.data) return;
    applyingRemote.current = true;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: query.data.schema_text } });
    applyingRemote.current = false;
    useSchemaDrafts.getState().removeDraft(connectionId);
    setConfirmDiscard(false);
    write.reset();
  };

  return <div className="flex h-full min-h-0 flex-col">
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-4 py-3 dark:border-slate-800">
      <div><h1>Schema</h1><p className="text-xs text-slate-500">
        {query.isFetching ? "Reading…" : query.data ? `Last read ${new Date(query.dataUpdatedAt).toLocaleTimeString()}` : "Not loaded"}
        {dirty && " · draft saved on this Mac"}
      </p></div>
      <div className="flex flex-wrap gap-2">
        {dirty && <Button variant="ghost" disabled={!query.data || write.isPending} onClick={() => setConfirmDiscard(true)}>Discard draft</Button>}
        <Button variant="secondary" disabled={query.isFetching} onClick={() => query.refetch()}>Refresh</Button>
        <Button disabled={!dirty || write.isPending || !query.data || query.isError} onClick={() => write.mutate(editor.current!.state.doc.toString())}>{write.isPending ? "Writing…" : "Write schema"}</Button>
      </div>
    </div>
    {query.error && <div className="p-3"><Banner tone="error">Failed to read schema: {query.error.message}</Banner></div>}
    {write.error && <div className="p-3"><Banner tone="error">Write failed: {write.error.message}</Banner></div>}
    {dirty && <div className="px-4 py-2 text-xs text-slate-600 dark:text-slate-400">Your draft survives navigation and restart. Refresh keeps your edits; Discard draft restores the last read version.</div>}
    <div ref={parent} className="min-h-0 flex-1 overflow-hidden" />
    {confirmDiscard && <Dialog title="Discard schema draft?" onClose={() => setConfirmDiscard(false)}>
      <p className="my-4 text-sm">Replace your local edits with the last schema read from this connection?</p>
      <div className="flex justify-end gap-2"><Button variant="secondary" onClick={() => setConfirmDiscard(false)}>Keep draft</Button><Button variant="danger" onClick={discard}>Discard draft</Button></div>
    </Dialog>}
  </div>;
}
