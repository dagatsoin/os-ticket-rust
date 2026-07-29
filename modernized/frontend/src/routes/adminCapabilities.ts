// Shared admin-area capability list (TS-M4-A0 / M4-PREP). Kept in its own module
// (not in guards.tsx) so both the route guards and the AppShell "Administration"
// nav entry can import it without tripping the react-refresh only-export-components
// rule (guards.tsx also exports components).
import type { StaffCapability } from "../stores/AuthStore";

/**
 * The delegated capability flags that grant entry to the admin area. An admin OR
 * a staff member holding ANY of these may reach /staff/admin (RequireAdminArea),
 * and the AppShell "Administration" nav entry mirrors this exact gate.
 */
export const DELEGATED_CAPABILITIES: StaffCapability[] = [
  "can_manage_faq",
  "can_manage_premade",
  "can_ban_emails",
  "can_view_staff_stats",
];
