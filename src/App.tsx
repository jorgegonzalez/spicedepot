import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { ConnectionsPage } from "./pages/ConnectionsPage";
import { PermissionsPage } from "./pages/PermissionsPage";
import { SchemaPage } from "./pages/SchemaPage";

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/connections" replace />} />
        <Route path="/connections" element={<ConnectionsPage />} />
        <Route path="/schema" element={<SchemaPage />} />
        <Route path="/permissions" element={<PermissionsPage />} />
      </Route>
    </Routes>
  );
}
