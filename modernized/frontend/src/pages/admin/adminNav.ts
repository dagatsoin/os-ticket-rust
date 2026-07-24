// Data-driven admin sidebar nav (TS-M4-A0). Every M4 destination is listed here so
// later epics add an entry + a route, never restructure the shell. Entries with a
// `requiredCapability` are delegated screens visible to a non-admin who holds that
// flag; all other entries are admin-only. `exact` marks the index (System Settings)
// so it is only "selected" on an exact path match.
import type { StaffCapability } from "../../stores/AuthStore";

export interface AdminNavItem {
  label: string;
  path: string;
  /** Delegated screens: shown when the staff member holds this flag (or is admin). */
  requiredCapability?: StaffCapability;
  /** Highlight only on an exact path match (used for the index route). */
  exact?: boolean;
}

export const ADMIN_NAV: AdminNavItem[] = [
  { label: "System Settings", path: "/staff/admin", exact: true },
  { label: "Staff", path: "/staff/admin/staff" },
  { label: "Groups", path: "/staff/admin/groups" },
  { label: "Departments", path: "/staff/admin/departments" },
  { label: "Teams", path: "/staff/admin/teams" },
  { label: "Help Topics", path: "/staff/admin/help-topics" },
  { label: "SLA", path: "/staff/admin/sla" },
  { label: "Priorities", path: "/staff/admin/priorities" },
  { label: "Site Pages", path: "/staff/admin/pages" },
  { label: "Logs", path: "/staff/admin/logs" },
  { label: "FAQ Categories", path: "/staff/admin/faq-categories", requiredCapability: "can_manage_faq" },
  { label: "Canned Responses", path: "/staff/admin/canned", requiredCapability: "can_manage_premade" },
];
