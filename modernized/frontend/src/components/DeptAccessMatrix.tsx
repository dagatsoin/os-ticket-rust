// Department-access matrix (TS-M4-B4) — now a thin wrapper over the generalised
// AccessMatrix (TS-M4-C2). Kept as a named component so GroupListPage and its
// tests keep the original `dept-*` data-testids unchanged.
//
// @implements FS-031.8: department checkbox matrix with select-all / select-none.
// @implements BS-031-021: the department-access set is a full-replace sync.
import { AccessMatrix } from "./AccessMatrix";
import type { IdName } from "../stores/StaffAdminStore";

export interface DeptAccessMatrixProps {
  /** All selectable departments. */
  departments: IdName[];
  /** Predicate: is this department currently checked? */
  isChecked: (id: number) => boolean;
  /** Toggle a single department. */
  onToggle: (id: number) => void;
  /** Check every department. */
  onSelectAll: () => void;
  /** Uncheck every department. */
  onSelectNone: () => void;
  disabled?: boolean;
}

/**
 * Department checkbox matrix with bulk Select All / Select None controls.
 * @implements FS-031.8 / BS-031-021.
 */
export function DeptAccessMatrix({
  departments,
  isChecked,
  onToggle,
  onSelectAll,
  onSelectNone,
  disabled,
}: DeptAccessMatrixProps) {
  return (
    <AccessMatrix
      items={departments}
      isChecked={isChecked}
      onToggle={onToggle}
      onSelectAll={onSelectAll}
      onSelectNone={onSelectNone}
      testIdPrefix="dept"
      emptyLabel="No departments available."
      disabled={disabled}
    />
  );
}
