import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { ConnectionsPage } from "./pages/ConnectionsPage";

// Connections is always the first page hit (and the redirect target), so we
// keep it in the main chunk. The rest split out — most importantly SchemaPage,
// which pulls in CodeMirror (the largest single dependency).
const SchemaPage = lazy(() =>
  import("./pages/SchemaPage").then((m) => ({ default: m.SchemaPage })),
);
const PermissionsPage = lazy(() =>
  import("./pages/PermissionsPage").then((m) => ({ default: m.PermissionsPage })),
);
const LookupPage = lazy(() =>
  import("./pages/LookupPage").then((m) => ({ default: m.LookupPage })),
);
const RelationshipsPage = lazy(() =>
  import("./pages/RelationshipsPage").then((m) => ({
    default: m.RelationshipsPage,
  })),
);
const WatchPage = lazy(() =>
  import("./pages/WatchPage").then((m) => ({ default: m.WatchPage })),
);

function PageLoading() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-slate-500">
      Loading…
    </div>
  );
}

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/connections" replace />} />
        <Route path="/connections" element={<ConnectionsPage />} />
        <Route
          path="/schema"
          element={
            <Suspense fallback={<PageLoading />}>
              <SchemaPage />
            </Suspense>
          }
        />
        <Route
          path="/permissions"
          element={
            <Suspense fallback={<PageLoading />}>
              <PermissionsPage />
            </Suspense>
          }
        />
        <Route
          path="/lookup"
          element={
            <Suspense fallback={<PageLoading />}>
              <LookupPage />
            </Suspense>
          }
        />
        <Route
          path="/relationships"
          element={
            <Suspense fallback={<PageLoading />}>
              <RelationshipsPage />
            </Suspense>
          }
        />
        <Route
          path="/watch"
          element={
            <Suspense fallback={<PageLoading />}>
              <WatchPage />
            </Suspense>
          }
        />
      </Route>
    </Routes>
  );
}
