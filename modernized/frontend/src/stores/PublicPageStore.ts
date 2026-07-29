// Public site-page store: fetches an active `type='other'` page by slug via the
// UNAUTHENTICATED GET /api/pages/:slug endpoint (rides the client apiClient, but
// no session is required). Powers the public /pages/:slug viewer. A 404 is
// surfaced as a distinct `notFound` state (its own UI branch); other failures
// surface a generic `error`.
//
// @implements FS-033: public serving of a custom `other` page by slug.
import { makeAutoObservable, runInAction } from "mobx";
import { ApiClient } from "../api/apiClient";
import { ApiError } from "../api/types";

/** A rendered public page: display name + admin-authored HTML body. */
export interface PublicPage {
  name: string;
  body: string;
}

export class PublicPageStore {
  page: PublicPage | null = null;
  loading = false;
  notFound = false;
  error: string | null = null;

  constructor(private readonly api: ApiClient) {
    makeAutoObservable<PublicPageStore, "api">(this, { api: false }, { autoBind: true });
  }

  /** Fetch the page for `slug`, resetting prior state. */
  async load(slug: string): Promise<void> {
    this.loading = true;
    this.error = null;
    this.notFound = false;
    this.page = null;
    try {
      const data = await this.api.get<PublicPage>(`/api/pages/${encodeURIComponent(slug)}`);
      runInAction(() => {
        this.page = data;
      });
    } catch (e) {
      runInAction(() => {
        if (e instanceof ApiError && e.status === 404) {
          this.notFound = true;
        } else {
          this.error = e instanceof ApiError ? e.message : String(e);
        }
      });
    } finally {
      runInAction(() => {
        this.loading = false;
      });
    }
  }
}
