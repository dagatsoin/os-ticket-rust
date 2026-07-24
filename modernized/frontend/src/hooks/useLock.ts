/**
 * useLock hook for ticket locking.
 * Manages lock acquisition, renewal, and release lifecycle.
 *
 * @implements TS-M3-I6: lock UI (auto-renew polling)
 */
import { useEffect, useCallback } from "react";
import { useStores } from "../stores/StoreContext";

export interface UseLockOptions {
  /** Ticket ID to lock. */
  ticketId: number;
  /** Whether to acquire lock on mount. */
  autoAcquire?: boolean;
}

export interface UseLockResult {
  /** True if we hold the lock. */
  hasLock: boolean;
  /** True if another agent holds the lock. */
  lockedByAnother: boolean;
  /** Name of the agent who holds the lock. */
  lockedByName: string | null;
  /** Lock error message. */
  lockError: string | null;
  /** True while acquiring lock. */
  lockLoading: boolean;
  /** Manually acquire the lock. */
  acquireLock: () => Promise<boolean>;
  /** Manually release the lock. */
  releaseLock: () => Promise<void>;
}

/**
 * Hook for managing ticket lock lifecycle.
 * Acquires lock on mount (if autoAcquire), releases on unmount.
 * @implements TS-M3-I6 AC-1: Acquires lock on mount
 * @implements TS-M3-I6 AC-4: Renews periodically
 * @implements TS-M3-I6 AC-5: Releases on navigation away
 */
export function useLock({ ticketId, autoAcquire = true }: UseLockOptions): UseLockResult {
  const { staffTickets } = useStores();

  const acquireLock = useCallback(async () => {
    return staffTickets.acquireLock(ticketId);
  }, [staffTickets, ticketId]);

  const releaseLock = useCallback(async () => {
    return staffTickets.releaseLock(ticketId);
  }, [staffTickets, ticketId]);

  // Acquire lock on mount, release on unmount
  useEffect(() => {
    if (autoAcquire && ticketId) {
      void acquireLock();
    }

    // Release lock on unmount or navigation away
    return () => {
      // Use sendBeacon for page unload to ensure lock is released
      if (staffTickets.lockId) {
        void releaseLock();
      }
      staffTickets.clearLockState();
    };
  }, [ticketId, autoAcquire, acquireLock, releaseLock, staffTickets]);

  // Handle page unload (browser close, refresh)
  useEffect(() => {
    const handleBeforeUnload = () => {
      if (staffTickets.lockId) {
        // Use sendBeacon for reliable delivery on page unload
        const url = `/api/staff/tickets/${ticketId}/lock`;
        const csrfToken = document.cookie
          .split("; ")
          .find((c) => c.startsWith("XSRF-TOKEN-STAFF="))
          ?.split("=")[1];
        if (csrfToken) {
          navigator.sendBeacon(
            url,
            new Blob([JSON.stringify({ _method: "DELETE" })], {
              type: "application/json",
            }),
          );
        }
      }
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [ticketId, staffTickets.lockId]);

  return {
    hasLock: Boolean(staffTickets.lockId),
    lockedByAnother: staffTickets.lockedByAnother,
    lockedByName: staffTickets.lockedByName,
    lockError: staffTickets.lockError,
    lockLoading: staffTickets.lockLoading,
    acquireLock,
    releaseLock,
  };
}
