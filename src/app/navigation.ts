import {
  AppWindow,
  Boxes,
  Code2,
  Copy,
  Cpu,
  Download,
  HelpCircle,
  LayoutDashboard,
  Library,
  Package,
  Plug,
  Server,
  Settings as SettingsIcon,
  Terminal,
  Type,
} from "lucide-react";
import type { ComponentType } from "react";

import type { SoftwareCategory } from "@/types/domain";

export interface NavItem {
  label: string;
  to: string;
  icon: ComponentType<{ className?: string }>;
  /** When set, this nav item pre-filters the inventory browser to these categories. */
  categories?: SoftwareCategory[];
}

/**
 * Categories with no dedicated sidebar entry (Theme, IconPack,
 * Documentation, LanguagePack) are still reachable via "All Software" and
 * via search/filters (M4.5c/e) — the nav list intentionally mirrors the
 * product spec's fixed navigation rather than enumerating every category.
 */
export const NAV_ITEMS: NavItem[] = [
  { label: "Dashboard", to: "/", icon: LayoutDashboard },
  { label: "All Software", to: "/inventory", icon: Package },
  {
    label: "Applications",
    to: "/inventory/application",
    icon: AppWindow,
    categories: ["application"],
  },
  {
    label: "Command-line Tools",
    to: "/inventory/command-line-tool",
    icon: Terminal,
    categories: ["commandLineTool"],
  },
  { label: "Libraries", to: "/inventory/library", icon: Library, categories: ["library"] },
  { label: "Fonts", to: "/inventory/font", icon: Type, categories: ["font"] },
  { label: "Runtimes", to: "/inventory/runtime", icon: Cpu, categories: ["runtime"] },
  {
    label: "Development",
    to: "/inventory/development",
    icon: Code2,
    categories: ["developmentPackage"],
  },
  {
    label: "System",
    to: "/inventory/system",
    icon: Server,
    categories: ["systemService", "kernelComponent", "driver", "firmware", "desktopComponent"],
  },
  {
    label: "Miscellaneous",
    to: "/inventory/miscellaneous",
    icon: Boxes,
    categories: ["miscellaneous"],
  },
  {
    label: "Unclassified",
    to: "/inventory/unclassified",
    icon: HelpCircle,
    categories: ["unclassified"],
  },
  { label: "Duplicates", to: "/duplicates", icon: Copy },
  { label: "Export", to: "/export", icon: Download },
  { label: "Providers", to: "/providers", icon: Plug },
  { label: "Settings", to: "/settings", icon: SettingsIcon },
];
