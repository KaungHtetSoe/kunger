import type {
  ClassificationConfidence,
  InstallationReason,
  InstallationScope,
  PackageManager,
} from "@/types/domain";

export interface InventoryFilters {
  packageManagers: PackageManager[];
  scopes: InstallationScope[];
  installationReasons: InstallationReason[];
  updateAvailableOnly: boolean;
  minConfidence: ClassificationConfidence | null;
}

export const EMPTY_FILTERS: InventoryFilters = {
  packageManagers: [],
  scopes: [],
  installationReasons: [],
  updateAvailableOnly: false,
  minConfidence: null,
};
