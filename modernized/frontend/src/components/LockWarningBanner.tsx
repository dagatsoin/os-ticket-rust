/**
 * Lock warning banner component.
 * Shows warning when ticket is locked by another agent.
 *
 * @implements TS-M3-I6: lock UI warning banner
 */
import { observer } from "mobx-react-lite";
import { Alert, AlertTitle } from "@mui/material";
import { Lock, Warning } from "@mui/icons-material";

export interface LockWarningBannerProps {
  /** True if ticket is locked by another agent. */
  lockedByAnother: boolean;
  /** Name of the staff member who holds the lock. */
  lockedByName: string | null;
  /** Error message if lock acquisition failed. */
  lockError: string | null;
}

/**
 * Lock warning banner showing lock status.
 * @implements TS-M3-I6 AC-2: Warning banner when locked by another
 * @implements TS-M3-I6 AC-7: Graceful degradation warning
 */
export const LockWarningBanner = observer(function LockWarningBanner({
  lockedByAnother,
  lockedByName,
  lockError,
}: LockWarningBannerProps) {
  if (lockError) {
    return (
      <Alert
        severity="warning"
        icon={<Warning />}
        sx={{ mb: 2 }}
        data-testid="lock-error-banner"
      >
        <AlertTitle>Lock Warning</AlertTitle>
        {lockError}
      </Alert>
    );
  }

  if (lockedByAnother && lockedByName) {
    return (
      <Alert
        severity="warning"
        icon={<Lock />}
        sx={{ mb: 2 }}
        data-testid="lock-warning-banner"
      >
        <AlertTitle>Ticket Locked</AlertTitle>
        This ticket is currently locked by <strong>{lockedByName}</strong>.
        You can view the ticket, but you cannot reply or post notes until the lock is released.
      </Alert>
    );
  }

  return null;
});
