import { beforeEach, describe, expect, it } from "vitest";
import { SnackbarStore } from "./SnackbarStore";

describe("SnackbarStore (TS-M4-A0)", () => {
  let snackbar: SnackbarStore;

  beforeEach(() => {
    snackbar = new SnackbarStore();
  });

  it("starts closed", () => {
    expect(snackbar.open).toBe(false);
    expect(snackbar.message).toBe("");
  });

  it("success() shows a green message", () => {
    snackbar.success("System settings updated");
    expect(snackbar.open).toBe(true);
    expect(snackbar.severity).toBe("success");
    expect(snackbar.message).toBe("System settings updated");
  });

  it("error() shows a red message", () => {
    snackbar.error("Denied");
    expect(snackbar.open).toBe(true);
    expect(snackbar.severity).toBe("error");
    expect(snackbar.message).toBe("Denied");
  });

  it("close() dismisses while keeping the last message", () => {
    snackbar.success("Saved");
    snackbar.close();
    expect(snackbar.open).toBe(false);
    expect(snackbar.message).toBe("Saved");
  });
});
