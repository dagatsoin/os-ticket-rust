// Shared success/error notice store (TS-M4-A0). A single global snackbar reused by
// every M4 admin CRUD screen: settings saves, guard denials, and later epics. The
// store holds the current message + severity + open flag; <AppSnackbar/> renders it.
import { makeAutoObservable } from "mobx";

export type SnackbarSeverity = "success" | "error" | "info";

export class SnackbarStore {
  open = false;
  message = "";
  severity: SnackbarSeverity = "info";

  constructor() {
    makeAutoObservable(this, {}, { autoBind: true });
  }

  /** Show a notice; replaces any currently-visible message. */
  show(message: string, severity: SnackbarSeverity = "info"): void {
    this.message = message;
    this.severity = severity;
    this.open = true;
  }

  /** Convenience: a green success notice (e.g. "System settings updated"). */
  success(message: string): void {
    this.show(message, "success");
  }

  /** Convenience: a red error / denial notice. */
  error(message: string): void {
    this.show(message, "error");
  }

  /** Dismiss the current notice. */
  close(): void {
    this.open = false;
  }
}
