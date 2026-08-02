import type { PackageManager, SoftwareCategory } from "@/types/domain";

const PACKAGE_MANAGER_LABELS: Record<PackageManager, string> = {
  apt: "APT",
  flatpak: "Flatpak",
  snap: "Snap",
  appImage: "AppImage",
  pip: "pip",
  pipx: "pipx",
  npm: "npm",
  cargo: "Cargo",
  manual: "Manual",
  unknown: "Unknown",
};

export function formatPackageManager(manager: PackageManager): string {
  return PACKAGE_MANAGER_LABELS[manager] ?? manager;
}

const SOFTWARE_CATEGORY_LABELS: Record<SoftwareCategory, string> = {
  application: "Application",
  commandLineTool: "Command-line Tool",
  library: "Library",
  font: "Font",
  runtime: "Runtime",
  developmentPackage: "Development Package",
  theme: "Theme",
  iconPack: "Icon Pack",
  firmware: "Firmware",
  driver: "Driver",
  kernelComponent: "Kernel Component",
  systemService: "System Service",
  desktopComponent: "Desktop Component",
  documentation: "Documentation",
  languagePack: "Language Pack",
  miscellaneous: "Miscellaneous",
  unclassified: "Unclassified",
};

export function formatSoftwareCategory(category: SoftwareCategory): string {
  return SOFTWARE_CATEGORY_LABELS[category] ?? category;
}

/** Auto-compact large counts for stat tiles: 1284 -> "1,284", 12900 -> "12.9K". */
export function formatCount(value: number): string {
  if (value < 10_000) {
    return value.toLocaleString();
  }
  if (value < 1_000_000) {
    return `${(value / 1000).toFixed(1)}K`;
  }
  return `${(value / 1_000_000).toFixed(1)}M`;
}
