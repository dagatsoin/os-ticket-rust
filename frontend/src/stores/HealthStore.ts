// Backend health indicator state. Polls GET /api/health and exposes a coarse
// "ok" / "down" / "unknown" status for the shell indicator (EPIC-M1-A AC-2).
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";

export type HealthStatus = "unknown" | "ok" | "down";

interface HealthResponse {
  status?: string;
}

export class HealthStore {
  private readonly api: ApiClient;
  status: HealthStatus = "unknown";

  constructor(api: ApiClient) {
    this.api = api;
    makeAutoObservable(this, {}, { autoBind: true });
  }

  /** Fetch /api/health; map a 2xx { status: "ok" } to "ok", anything else to "down". */
  async check(): Promise<void> {
    try {
      const data = await this.api.get<HealthResponse>("/api/health");
      runInAction(() => {
        this.status = data?.status === "ok" ? "ok" : "down";
      });
    } catch {
      runInAction(() => {
        this.status = "down";
      });
    }
  }
}
