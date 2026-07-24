// Client-side admin route guards (TS-M4-A0). All three share one gate:
//   - while the /me profile is still resolving → render a spinner (no redirect flash);
//   - unauthenticated → redirect to the staff login;
//   - authenticated but not permitted → redirect to /staff/tickets with a denial snackbar;
//   - permitted → render the wrapped screen.
//
// `RequireAdmin`   — strict isadmin (System Settings, Attachments, admin-only CRUD).
// `RequireCapability` — isadmin OR a specific delegated flag (FAQ / Canned, EPIC-M4-E/H).
// `RequireAdminArea`  — isadmin OR ANY delegated flag; wraps the whole AdminLayout so a
//                        delegated non-admin can reach their own screen without denial.
import type { ReactNode } from "react";
import { useEffect } from "react";
import { observer } from "mobx-react-lite";
import { Navigate } from "react-router-dom";
import { Box, CircularProgress } from "@mui/material";
import { useStores } from "../stores/StoreContext";
import type { AuthStore, StaffCapability } from "../stores/AuthStore";

/** The delegated capability flags that grant entry to the admin area (TS-M4-A0 / M4-PREP). */
const DELEGATED_CAPABILITIES: StaffCapability[] = [
  "can_manage_faq",
  "can_manage_premade",
  "can_ban_emails",
  "can_view_staff_stats",
];

const DENIAL_MESSAGE = "You don't have permission to access the admin area.";

function GateSpinner() {
  return (
    <Box sx={{ display: "flex", justifyContent: "center", py: 8 }} data-testid="admin-gate-spinner">
      <CircularProgress />
    </Box>
  );
}

/**
 * Shared gate. `predicate` decides whether an authenticated staff member is
 * permitted; the profile-loading / unauthenticated branches are common.
 */
const AuthGate = observer(function AuthGate({
  predicate,
  children,
}: {
  predicate: (auth: AuthStore) => boolean;
  children: ReactNode;
}) {
  const { staffAuth, snackbar } = useStores();

  useEffect(() => {
    staffAuth.ensureProfileLoaded();
  }, [staffAuth]);

  const resolved = staffAuth.profileLoaded !== "loading";
  const allowed = predicate(staffAuth);
  const denied = resolved && staffAuth.isAuthenticated && !allowed;

  // Fire the denial notice as a side effect (never during render).
  useEffect(() => {
    if (denied) snackbar.error(DENIAL_MESSAGE);
  }, [denied, snackbar]);

  if (!resolved) return <GateSpinner />;
  if (!staffAuth.isAuthenticated) return <Navigate to="/staff/login" replace />;
  if (!allowed) return <Navigate to="/staff/tickets" replace />;
  return <>{children}</>;
});

/** Strict admin gate — only `isadmin` staff may pass. */
export function RequireAdmin({ children }: { children: ReactNode }) {
  return <AuthGate predicate={(a) => a.isAdmin}>{children}</AuthGate>;
}

/** Delegated-capability gate — `isadmin` OR the named flag. */
export function RequireCapability({
  flag,
  children,
}: {
  flag: StaffCapability;
  children: ReactNode;
}) {
  return <AuthGate predicate={(a) => a.isAdmin || a.can(flag)}>{children}</AuthGate>;
}

/** Admin-area gate — `isadmin` OR any delegated capability (wraps the whole shell). */
export function RequireAdminArea({ children }: { children: ReactNode }) {
  return (
    <AuthGate predicate={(a) => a.isAdmin || DELEGATED_CAPABILITIES.some((f) => a.can(f))}>
      {children}
    </AuthGate>
  );
}

/**
 * Staff gate (TS-M4-B6) — ANY authenticated staff member, no capability required.
 * Wraps the non-admin own-profile + directory screens, which must be reachable by
 * a plain agent (RequireAdmin would wrongly bounce them to the queue).
 */
export function RequireStaff({ children }: { children: ReactNode }) {
  return <AuthGate predicate={() => true}>{children}</AuthGate>;
}
