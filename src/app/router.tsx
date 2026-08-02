import { createHashRouter } from "react-router-dom";

import { AppShell } from "@/app/AppShell";
import { DashboardPage } from "@/pages/DashboardPage";
import { DuplicatesPage } from "@/pages/DuplicatesPage";
import { ExportPage } from "@/pages/ExportPage";
import { InventoryCategoryPage } from "@/pages/InventoryCategoryPage";
import { NotFoundPage } from "@/pages/NotFoundPage";
import { ProvidersPage } from "@/pages/ProvidersPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { SoftwareDetailsPage } from "@/pages/SoftwareDetailsPage";

// Hash routing avoids relying on server-side URL rewrites: Tauri serves
// the frontend as static assets with no server to rewrite deep-linked
// paths back to index.html.
export const router = createHashRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: "inventory", element: <InventoryCategoryPage /> },
      { path: "inventory/:category", element: <InventoryCategoryPage /> },
      { path: "software/:id", element: <SoftwareDetailsPage /> },
      { path: "duplicates", element: <DuplicatesPage /> },
      { path: "export", element: <ExportPage /> },
      { path: "providers", element: <ProvidersPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "*", element: <NotFoundPage /> },
    ],
  },
]);
