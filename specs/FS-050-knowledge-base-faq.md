# FS-050: Knowledge Base & FAQ

## Overview

The Knowledge Base & FAQ specification defines osTicket's self-service article subsystem: a **public client-facing knowledge base** (browse categories, read published FAQ articles, full-text search, filter by help topic) and a **staff-side FAQ management** area (create / edit / publish / unpublish / delete FAQ articles, attach files, mark articles public or internal, associate them with help topics, and write internal notes). It also covers the hover-preview AJAX endpoint that renders a FAQ article snippet inline, and the system-wide enable/disable toggle that turns the public client interface on or off.

The subsystem is built around three durable concepts:
- **FAQ article** (`FAQ` domain class, `faq` table) — a question/answer pair belonging to exactly one category, with a published flag, optional keywords, internal notes, file attachments, and zero-or-more help-topic associations.
- **FAQ category** (`Category` domain class, `faq_category` table) — a named, described grouping of FAQ articles, flagged public or internal. **Category create/edit/delete (CRUD) is owned by FS-032** (Admin — system settings, SLA, priorities & categories, via `scp/categories.php` and `include/staff/categor*.inc.php`); this spec consumes the `Category` class only as the parent grouping that governs an article's effective public visibility and the breadcrumb/listing structure.
- **Help-topic association** (`faq_topic` join table) — a many-to-many link between FAQ articles and help topics (help topics themselves are owned by FS-030; the help-topic *entity* and `ispublic` flag are referenced, not defined, here).

Two visibility flags compose to decide whether a member of the public ever sees an article: the **article's `ispublished` flag** AND the **parent category's `ispublic` flag**. An article is only ever shown to the public when both are true (`isPublished()`); staff see all articles regardless of either flag (subject to the staff KB tab being visible).

> **Note**: The class `Knowledgebase` (`include/class.knowledgebase.php`) is present in the source slice but is effectively dead/legacy code: its constructor and all queries read/write the **canned-response** table (`CANNED_TABLE` / `canned_id`), not any FAQ table, and no entry script in this subsystem instantiates it. The live staff-side knowledge base in osTicket 1.7 is the FAQ subsystem (`FAQ` + `Category` classes). See KL-050.5. Canned responses proper are owned by FS-022.

> **Note**: Column/table schemas for `faq`, `faq_category`, `faq_attachment`, `faq_topic`, and the `enable_kb` config key are canonically defined in **FS-091** (Reference data, enums & data model). This spec references field and table names as behavioral facts but does not restate the full schema.

---

## Functional Requirements

### FS-050.1: Knowledge-Base Enable/Disable Toggle

**Description**: The system shall provide a single administrator-controlled toggle (`enable_kb`) that governs whether the public (client-facing) knowledge-base interface is reachable.

**Acceptance Criteria**:
- The toggle is exposed on the admin Knowledge Base Settings page (`settings.php?t=kb`, view `include/staff/settings-kb.inc.php`) as a checkbox labelled **"Enable Knowledge base (Client interface)"**, with the section caption **"Disabling knowledge base disables clients' interface."** (The settings page itself and persistence of `enable_kb` are owned by FS-032/FS-033; only its effect on KB reachability is in scope here.)
- The config accessor `isKnowledgebaseEnabled()` returns true only when **both** `enable_kb` is set **and** `FAQ::countPublishedFAQs()` returns a non-zero count (at least one published article in a public category exists). It returns false otherwise.
- When `isKnowledgebaseEnabled()` is false, the public "Knowledgebase" link is omitted from the client-portal navigation (`UserNav`), and every public KB entry script (`kb/index.php`, `kb/faq.php`) redirects the visitor away (see FS-050.2, BS-050.1).
- The toggle has no effect on the staff-side FAQ area: staff can manage and view FAQ articles even when the public KB is disabled (the staff KB tab is gated by staff permission and group nav, not by `enable_kb`).

### FS-050.2: Public Knowledge-Base Entry & Guard

**Description**: The system shall expose the public knowledge base through a small set of entry scripts that all share a common guard, redirecting visitors away when the KB is unavailable.

**Acceptance Criteria**:
- Every public KB script includes the shared bootstrap `kb/kb.inc.php`, which loads the client request context (`client.inc.php`) and the FAQ class, then evaluates the guard.
- The guard redirects the visitor to the support-center root (`../`) and stops execution when **any** of the following is true: configuration is unavailable (`$cfg` is falsy), the KB is disabled (`isKnowledgebaseEnabled()` false), or there are zero published public FAQs (`FAQ::countPublishedFAQs()` is zero).
- When the guard passes, it constructs a client navigation object with the `kb` tab active (`UserNav($thisclient, 'kb')`).
- The public KB landing page (`kb/index.php`) renders, in order, the client header, the knowledge-base body (`knowledgebase.inc.php`), and the client footer.
- The public article/category page (`kb/faq.php`) accepts an article id (`id`) or category id (`cid`) request parameter and selects which body partial to render (see FS-050.3).

### FS-050.3: Public Article / Category Routing (`kb/faq.php`)

**Description**: The public FAQ script shall resolve the requested article or category and choose the appropriate view, falling back to the landing page when the target is missing or not publicly visible.

**Acceptance Criteria**:
- If an `id` parameter is present, the script attempts `FAQ::lookup(id)`; if lookup fails it records the error **"Unknown or invalid FAQ"** and `$faq` remains null.
- If no article resolved and a `cid` parameter is present, it attempts `Category::lookup(cid)`; if lookup fails it records **"Unknown or invalid FAQ category"**. **Observed behavior**: `Category::lookup` returns a (possibly hollow) object for *any* numeric `cid` — its success test is `is_numeric($id) && (new Category($id))`, which never short-circuits on a missing row (unlike `FAQ::lookup`, which additionally checks `getId()==$id`). A non-existent numeric `cid` therefore yields a hollow `Category` whose `getId()` is `0` and whose `isPublic()` is null/falsy, so the "Unknown or invalid FAQ category" error is effectively unreachable for numeric input; the request silently falls through to the landing page. See KL-050.8.
- View selection (default `knowledgebase.inc.php`, the landing page):
  - If a FAQ resolved **and** it is published (`$faq->isPublished()` — article published AND category public), render the single-article view `faq.inc.php`.
  - Else, if a category resolved **and** it is public (`$category->isPublic()`) **and** the action is not `search` (`a != 'search'`), render the category-listing view `faq-category.inc.php`.
  - Otherwise, render the landing page `knowledgebase.inc.php`.
- The chosen body is wrapped by the client header and footer.
- A resolved-but-unpublished article (article internal, or category not public) does **not** render the single-article view; it falls through to the landing page (the article is invisible to the public). The single-article partial additionally self-guards with `die('Access Denied')` if `$faq` is absent or not published.
- The category-listing partial self-guards with `die('Access Denied')` if `$category` is absent or not public.

### FS-050.4: Public Landing Page — Category Listing & Search Form (`knowledgebase.inc.php`)

**Description**: The public landing page shall present a search form (free-text query, category filter, help-topic filter) and, in the absence of a search, a listing of public categories that contain at least one published article.

**Acceptance Criteria**:
- The page heading is **"Frequently Asked Questions"**.
- A search form (`#kb-search`, GET to `index.php`) carries a hidden `a=search` marker and contains:
  - A free-text query input (`q`), pre-filled with the current escaped query.
  - A category filter select (`cid`) whose first option is **"— All Categories —"** (empty value), followed by one option per **public** category that has at least one **published** article, each showing the category name and its published-article count, e.g. `Billing (4)`. Options are ordered by category name descending.
  - A **"Search"** submit button.
  - A help-topic filter select (`topicId`) whose first option is **"— All Help Topics —"** (empty value), followed by one option per **public** help topic that is associated with at least one FAQ, showing the topic path (`parent / child`) and FAQ count, ordered by that label.
- **Listing mode** (no `q`, no `cid`, no `topicId`): the page lists every public category with at least one published article (`HAVING faqs>0`), ordered by name ascending. Each list item shows the category name with its published-article count as a link to `faq.php?cid=<id>`, plus the (safe-HTML) category description. Instructional copy reads **"Click on the category to browse FAQs."** If no such category exists, it prints **"NO FAQs found"**.
- **Search mode** (any of `q`, `cid`, `topicId` present): see FS-050.5.

### FS-050.5: Public FAQ Search

**Description**: When a search is active, the public landing page shall return a numbered list of matching published, public articles.

**Acceptance Criteria**:
- The result set is restricted to articles that are published **and** whose category is public (`faq.ispublished=1 AND cat.ispublic=1`).
- Filters compose with AND:
  - `cid` restricts to a single category.
  - `topicId` restricts to articles associated with that help topic (via the `faq_topic` join).
  - `q` matches (case-insensitively, as a substring) against any of: the article **question**, the article **answer**, the article **keywords**, the category **name**, or the category **description**.
- Results are grouped by article and ordered by question; each is rendered as a numbered list item linking to `faq.php?id=<id>` with the CSS hook `previewfaq` (enables the hover preview, FS-050.10). **Observed behavior**: the public result template carries a trailing `Published`/`Internal` annotation argument referencing `$row['ispublished']`, but the public search query selects only `faq_id, question` (not `ispublished`), so the annotation never renders — it is a no-op artifact. (Public results are by definition all published/public.) See KL-050.9.
- A header **"Search Results"** precedes the list; a non-empty result set is introduced by **"N FAQs matched your search criteria."**
- An empty result set prints **"The search did not match any FAQs."**

### FS-050.6: Public Category View (`faq-category.inc.php`)

**Description**: The public category view shall display a single public category's name, description, and a numbered list of its published articles.

**Acceptance Criteria**:
- The view renders only when the category is public; otherwise `die('Access Denied')`.
- It shows the category name as a heading and the safe-HTML category description.
- It lists every **published** article in that category (`faq.ispublished=1`), ordered by question, each as a numbered link to `faq.php?id=<id>`. Articles with attachments display a file icon marker (`<span class="Icon file">`).
- A **"Go Back"** link returns to the landing page (`index.php`).
- If the category has no published articles, it prints **"Category does not have any FAQs."** with a back-to-index link.

### FS-050.7: Public Single-Article View (`faq.inc.php`)

**Description**: The public single-article view shall display one published article with breadcrumbs, answer, attachments, and associated help topics.

**Acceptance Criteria**:
- The view renders only when the article is published; otherwise `die('Access Denied')`.
- A breadcrumb trail links **"All Categories"** (→ `index.php`) and the parent category name (→ `faq.php?cid=<catId>`).
- The article question is shown as a prominent heading; the answer is rendered as safe HTML.
- If the article has attachments, an **"Attachments:"** label is shown with attachment download links (see FS-050.9).
- A **"Help Topics:"** line lists the comma-separated names of the article's associated help topics (blank when none).
- A "Last updated" line is shown with a formatted date.

> **Note (observed behavior)**: The public single-article view's "Last updated" date reads the **parent category's** update timestamp (`$category->getUpdateDate()`), not the article's own update timestamp. The staff single-article view (FS-050.13) correctly uses the article's own timestamp. See KL-050.2.

### FS-050.8: Staff FAQ Management Permission Gate

**Description**: The system shall gate all staff-side FAQ creation, editing, publishing, and deletion behind a per-group "can manage FAQ" permission, while allowing any staff member with the KB tab to read articles.

**Acceptance Criteria**:
- A staff member's FAQ-management capability is determined by `canManageFAQ()`, which reflects the staff member's group `can_manage_faq` permission flag (group permissions are owned by FS-031; default seeded value is off / `0`). `canManageFAQs()` is an alias of `canManageFAQ()`.
- The staff FAQ edit/add form (`include/staff/faq.inc.php`) self-guards: it renders only for an authenticated staff member with `canManageFAQ()`; otherwise `die('Access Denied')`.
- The staff category-listing/search view shows management affordances (Edit Category, Delete Category, Add New FAQ, the per-article manage panel) only when `canManageFAQ()` is true; non-managers see read-only listings.
- The "Categories" sub-tab in the staff KB navigation is shown only to staff with `canManageFAQ()`.
- All POST mutation actions on `scp/faq.php` that publish/unpublish/delete/edit additionally require `canManageFAQ()` before the corresponding edit form is shown (`a=edit` and `a=add` only switch to the edit form when `canManageFAQ()` is true).

### FS-050.9: FAQ Article Attachments

**Description**: The system shall allow FAQ articles to carry file attachments, uploaded on the create/edit form and removable by unchecking, and shall expose them as download links on article views.

**Acceptance Criteria**:
- An article may have zero or more attachments, stored as join rows in `faq_attachment` referencing content-addressed files (file storage is owned by FS-022/FS-091; this spec governs the FAQ ↔ file association only).
- On article create (`add`) and update, any files posted under the `attachments[]` field are uploaded and linked to the article. The upload helper (`uploadAttachments`) also accepts already-stored file **ids** directly: each item is treated as an existing file id when numeric, otherwise it is uploaded first (`is_numeric($file) ? $file : AttachmentFile::upload($file)`); a non-numeric, failed-upload item is skipped. A join row is inserted only for a numeric resulting file id, and the article is reloaded only if at least one attachment was linked.
- On update, the form submits the ids of attachments to **keep** (the checked `files[]` checkboxes); any existing attachment whose id is not in the keep-list is detached and, if it becomes orphaned, the underlying file is reclaimed (`deleteAttachment` → `AttachmentFile::deleteOrphans`).
- Deleting an article removes all of its attachment join rows and reclaims any orphaned files.
- Attachment download links are generated against `file.php?h=<hash>` where the hash is the file content hash concatenated with a per-session validation token (`md5(id + session_id + hash)`); a human-readable file size is shown when known.
- The number of attachments per article is surfaced via `getNumAttachments()` and shown as a count in preview/management contexts.

### FS-050.10: Inline FAQ Preview (AJAX)

**Description**: The system shall provide an AJAX endpoint that returns a compact HTML snippet of a FAQ article for inline hover-preview, and shall trigger it from any link carrying the `previewfaq` CSS hook.

**Acceptance Criteria**:
- The endpoint `ajax.php/kb/faq/<id>` (handled by `KbaseAjaxAPI::faq`) returns an HTML fragment containing the article question, the safe-HTML answer, a "Last updated <date>" line, and two links: **"View"** (→ `faq.php?id=<id>`) and **"Attachments (N)"**.
- If the requesting context is a staff member with `canManageFAQ()`, the fragment additionally includes an **"Edit"** link (→ `faq.php?id=<id>&a=edit`).
- If the requested article id does not resolve, the endpoint returns null (no fragment).
- Any link with class `previewfaq` triggers the preview on hover after an ~750 ms delay; moving the pointer away before the delay cancels it. The preview id is derived from the link's `id=`/`cid=` query value.
- The same AJAX controller also exposes a canned-response retrieval method (`cannedResp`) under the `kb` AJAX namespace; that behavior is owned by FS-022 and only noted here as co-located. (It returns HTTP 404 **"No such premade reply"** for an unknown/disabled canned id.)

### FS-050.11: Staff KB Entry & Routing (`scp/kb.php`, `scp/faq.php`)

**Description**: The staff control panel shall expose the FAQ area through two entry scripts: a category/landing browser (`kb.php`) and an article view/management script (`faq.php`), both activating the `kbase` navigation tab.

**Acceptance Criteria**:
- `scp/kb.php` resolves an optional `cid` (recording **"Unknown or invalid FAQ category"** on failure) and selects: the category-detail view `faq-category.inc.php` when a category resolved and the action is not `search`; otherwise the categories landing/search view `faq-categories.inc.php`. It sets the `kbase` nav tab active and wraps the body in the staff header/footer.
- `scp/faq.php` resolves an optional article `id` (error **"Unknown or invalid FAQ"**) and/or category `cid` (error **"Unknown or invalid FAQ category"**), processes any POST mutation (FS-050.12), then selects a view (FS-050.13).
- The staff KB navigation sub-tabs (under the `kbase` tab) are: **"FAQs"** (→ `kb.php`, also matching `faq.php`), **"Categories"** (→ `categories.php`, FS-032, manager-only), and **"Canned Responses"** (→ `canned.php`, FS-022, gated by canned-response permission).

### FS-050.12: Staff FAQ Mutation Actions (`scp/faq.php` POST)

**Description**: On POST, the staff FAQ script shall dispatch on a `do` action to create, update, or manage an article, returning success or error messages.

**Acceptance Criteria**:
- `do = create` / `add`: validates and inserts a new article via `FAQ::add`; on success sets **"FAQ added successfully"**, otherwise **"Unable to add FAQ. Try again!"** (unless a field-level error was already set). On success, the returned new-article object is assigned to `$faq`, so the post-POST view selection (FS-050.13) lands on the new article's **management view** (`faq-view.inc.php`), not the form. **Class-level error strings**: `FAQ::save` (via `create`) sets its own `err` value **"Unable to create FAQ. Internal error"** if the INSERT itself fails; because the entry script only sets "Unable to add FAQ. Try again!" when `!$errors['err']`, that class message is the one surfaced on a DB-level create failure.
- `do = update` / `edit`: requires a resolved article (else **"Invalid or unknown FAQ"**); on success sets **"FAQ updated successfully"**, clears the edit action so the view returns to the article, and reloads; otherwise **"Unable to update FAQ. Try again!"** (unless a field-level error was already set). **Class-level error string**: `FAQ::save` sets its own `err` value **"Unable to update FAQ."** if the UPDATE itself fails at the DB level; that message is surfaced in preference to "Unable to update FAQ. Try again!".
- `do = manage-faq`: requires a resolved article (else **"Unknown or invalid FAQ"**), then dispatches on a nested `a` action:
  - `edit` → switch to the edit form (sets `a=edit`).
  - `publish` → `FAQ::publish()`; success **"FAQ published successfully"**, failure **"Unable to publish the FAQ. Try editing it."**
  - `unpublish` → `FAQ::unpublish()`; success **"FAQ unpublished successfully"**, failure **"Unable to unpublish the FAQ. Try editing it."**
  - `delete` → captures the parent category, then `FAQ::delete()`; success **"FAQ deleted successfully"** (article cleared), failure **"Unable to delete FAQ. Try again"**.
  - any other nested action → **"Invalid action"**.
- An unrecognized top-level `do` value sets **"Unknown action"**.
- All POST forms carry a CSRF token (`csrf_token()`); CSRF enforcement is owned by FS-002.

### FS-050.13: Staff View Selection & Single-Article View

**Description**: After any POST processing, the staff FAQ script shall select the article-management view, the edit form, the category detail, or the categories landing page.

**Acceptance Criteria**:
- View selection in `scp/faq.php` (default `faq-categories.inc.php`, the landing/search page):
  - If an article resolved → `faq-view.inc.php` (the single-article management view); but if `a=edit` and the staff member can manage FAQs → `faq.inc.php` (the edit form).
  - Else if `a=add` and the staff member can manage FAQs → `faq.inc.php` (the add form, pre-filled with the category if `cid` supplied).
  - Else if a category resolved and the action is not `search` → `faq-category.inc.php`.
- The staff single-article view (`faq-view.inc.php`) shows: breadcrumbs (**"All Categories"** → `kb.php`, then the category name with its `(Public)`/`(Internal)` marker), the question with a `(Published)` marker when published, the safe-HTML answer, attachment links, comma-separated help-topic names, and a "Last updated" line using the **article's** own update date.
- For managers, the staff single-article view shows an **"Edit FAQ"** affordance and an Options panel offering **Publish/Unpublish** (toggled by current state), **Edit FAQ**, and **Delete FAQ** actions (posted back as `do=manage-faq`).

### FS-050.14: Staff Category Listing & Search (`faq-categories.inc.php`)

**Description**: The staff KB landing page shall present a search form and, absent a search, a listing of all categories (public and internal) with their article counts.

**Acceptance Criteria**:
- The search form (`#kbSearch`, GET to `kb.php`, hidden `a=search`) mirrors the public form's fields (query `q`, category select `cid`, help-topic select `topicId`) but the category and help-topic option lists are **not** restricted to public/published — every category with FAQs and every help topic with FAQs appears (staff see internal content).
- **Search mode** (any of `q`, `cid`, `topicId`): returns matching articles with **no** public/published restriction (`WHERE 1`), composing `cid`, `topicId`, and the same five-column `q` substring match as the public search. Each result links to `faq.php?id=<id>` (with `previewfaq` hook) and is annotated **"Published"** or **"Internal"** per the article's `ispublished` flag.
- **Listing mode**: lists every category (no public filter) with its total article count, ordered by name; each is a link to `kb.php?cid=<id>` annotated `Public`/`Internal` with its safe-HTML description.
- Empty result/listing prints **"The search did not match any FAQs."** / **"NO FAQs found"** respectively.

### FS-050.15: Staff Category Detail (`faq-category.inc.php`)

**Description**: The staff category-detail view shall show one category's metadata, its full article list (published and internal), and management affordances for managers.

**Acceptance Criteria**:
- Shows the category name, its `(Public)`/`(Internal)` marker, a "Last updated" date, and the safe-HTML description.
- Lists **all** articles in the category (no `ispublished` filter), ordered by question, each linking to `faq.php?id=<id>` (with `previewfaq`) and annotated **"Published"**/**"Internal"**.
- For managers, shows a management bar with **"Edit Category"** (→ `categories.php?id=<id>`, FS-032), **"Delete Category"** (→ `categories.php`, FS-032), and **"Add New FAQ"** (→ `faq.php?cid=<id>&a=add`).
- If the category has no articles, prints **"Category does not have FAQs"**.

### FS-050.16: Staff FAQ Create/Edit Form (`faq.inc.php`)

**Description**: The staff FAQ form shall collect the question, category, listing type (public/internal), answer, attachments, help-topic associations, and internal notes, for both create and edit modes.

**Acceptance Criteria**:
- In edit mode the form title is **"Update FAQ: <question>"**, action `update`, submit label **"Save Changes"**, pre-filled from the article (including selected help topics); in create mode the title is **"Add New FAQ"**, action `create`, submit label **"Add FAQ"**, optionally pre-selecting a category passed via `cid`.
- Fields:
  - **Question** (required, single-line text).
  - **Category Listing** (required select of all categories, each annotated `Public`/`Internal`; first option "Select FAQ Category", value 0).
  - **Listing Type** radio: **"Public (publish)"** (`ispublished=1`) vs **"Internal (private)"** (`ispublished=0`), with the note "Published questions are listed on public knowledgebase if the parent category is public."
  - **Answer** (required, rich-text textarea).
  - **Attachments** (optional multi-file; in edit mode, existing attachments appear as checked checkboxes that are unchecked to delete on submit, per FS-050.9).
  - **Help Topics** (a checkbox per help topic; checked ones are associated).
  - **Internal Notes** (optional textarea).
- Field-level validation errors are echoed inline next to their fields; on a failed submit the form re-renders from POSTed values.
- The form posts as `multipart/form-data` with a CSRF token; a Reset and a Cancel (returns to the prior context) control are provided.

### FS-050.17: FAQ Validation Rules (`FAQ::save`)

**Description**: The system shall validate an FAQ article before persistence and reject duplicates and missing required fields.

**Acceptance Criteria**:
- The **question** is trimmed and stripped of tags. A blank question yields the error **"Question required"**.
- A question that duplicates an existing article's question (other than the one being edited) yields **"Question already exists"** (questions are effectively unique).
- A missing or unresolvable **category** yields **"Category is required"**.
- A blank **answer** yields **"FAQ answer is required"**.
- On edit, if the submitted hidden `id` does not match the article id being saved (`$id && $id != $vars['id']`), a top-level error **"Internal error. Try again"** is set and the save is rejected. (A defensive guard against a tampered/mismatched id field.)
- The answer is stored after safe-HTML sanitization (`Format::safe_html`); `created` is set on insert and `updated` on every save (both via `NOW()`).
- The **notes** field is stored **verbatim** (no `Format::safe_html` / tag-stripping applied to notes on save); only the answer is sanitized and only the question is tag-stripped.
- The `ispublished` flag defaults to `0` (internal) when not supplied.
- A `validation`-only call (`save(..., $validation=true)`) returns the boolean pass/fail without persisting; the live create/update paths do not pass it, so all real saves both validate and persist.

### FS-050.18: Help-Topic Association Management (`FAQ::updateTopics`)

**Description**: The system shall synchronize an article's help-topic associations on each save: adding newly-checked topics and removing unchecked ones.

**Acceptance Criteria**:
- On save, each submitted topic id not already associated is inserted into `faq_topic` (idempotent insert); any existing association whose topic id is not in the submitted set is deleted.
- If no topics are submitted, all of the article's topic associations are removed.
- Deleting an article also removes all of its `faq_topic` rows (and all attachment rows; see FS-050.9).
- The help-topic *entities* (their names, the `parent / child` path, and the `ispublic` flag that gates them in public filters) are owned by FS-030 and are referenced read-only here.

---

## Business Rules

### BS-050.1: Public Visibility Requires Published Article AND Public Category

**Rule**: A FAQ article is visible to the public only when the article's `ispublished` flag is set **and** its parent category's `ispublic` flag is set. Either flag being false hides the article from all public surfaces (landing listing, category view, single-article view, search, and the navigation link/count).

**Rationale**: Two independent flags allow an entire category to be staged internally (category internal) and individual articles to be drafted within a public category (article internal) without exposing partial content.

**Examples**:
- A published article in an internal category is invisible to the public; staff still see it.
- An internal (unpublished) article in a public category is invisible to the public; staff see it marked "Internal".
- `FAQ::isPublished()` returns true only when both `ispublished` and the joined category `ispublic` are true.

### BS-050.2: Public KB Reachability Requires Toggle AND At Least One Published Public FAQ

**Rule**: The public knowledge base is reachable only when `enable_kb` is on AND at least one published article exists in a public category. If either condition fails, the navigation link is hidden and all public KB entry scripts redirect to the support-center root.

**Rationale**: An empty knowledge base offers nothing to a visitor; the system suppresses the entire interface rather than presenting an empty page, and an administrator can hard-disable it regardless of content.

**Examples**:
- `enable_kb` on but zero published public FAQs → KB link hidden, `kb/index.php` redirects to `../`.
- `enable_kb` off with many published FAQs → KB still hidden and redirecting.

### BS-050.3: Staff Visibility Is Permission-Gated, Not Content-Gated

**Rule**: Staff access to FAQ content is governed by navigation/group access and the `can_manage_faq` permission, independent of the public `enable_kb` toggle and independent of article/category visibility flags. Any staff member with the KB tab can read all articles (public and internal); only managers (`canManageFAQ()`) can create, edit, publish/unpublish, or delete.

**Rationale**: Internal articles and notes exist precisely so staff can reference content the public cannot; management mutations are a privileged subset.

**Examples**:
- A non-manager staff sees internal articles and the "Internal" markers but no Edit/Delete/Add affordances.
- A manager sees the same content plus the full management panel and the Categories sub-tab.

### BS-050.4: Article Question Uniqueness

**Rule**: Two FAQ articles cannot share the same question text; an attempt to create or rename an article to a question already used by a different article is rejected with "Question already exists".

**Rationale**: The question is the article's human identifier and is matched in search; duplicates would be ambiguous.

### BS-050.5: An Article Belongs to Exactly One Category; Deleting a Category Deletes Its Articles

**Rule**: Every FAQ article references exactly one category. Deleting a category cascades to delete all FAQ articles in that category (via `Category::delete`).

**Rationale**: Categories are the sole organizational grouping for articles; an orphaned article would have no place in the listing or breadcrumb structure. (Category delete is invoked from FS-032; the cascade behavior is noted here because it destroys FAQ content.)

**Examples**:
- Deleting the "Billing" category permanently removes every FAQ in it, including their attachment and topic associations (the latter via the FAQ delete path is not invoked by the category cascade — see KL-050.4).

### BS-050.6: Search Is Case-Insensitive Substring Match Across Five Fields

**Rule**: A free-text KB query matches as a substring (case-insensitively) against the article question, answer, keywords, and the parent category's name and description. The public search additionally restricts to published, public articles; the staff search applies no such restriction.

**Rationale**: Matching multiple fields maximizes recall for self-service; restricting the public search enforces visibility rules.

### BS-050.7: Help-Topic Filters Show Only Public Topics on the Public Side

**Rule**: The public KB's help-topic filter lists only help topics flagged public (`ispublic=1`) that are associated with at least one FAQ; the staff filter lists every help topic associated with at least one FAQ regardless of the public flag.

**Rationale**: A public visitor must not be able to enumerate or filter by internal help topics.

---

## Data Requirements

> Full column/table definitions are owned by **FS-091**. The entities consumed by this subsystem are summarized functionally below.

### FAQ Article (`faq` table, `FAQ` class)

| Field (functional) | Role |
|--------------------|------|
| `faq_id` | Article identifier. |
| `category_id` | Owning category (exactly one; required). |
| `question` | The question text — required, tag-stripped, unique across articles, search-indexed. |
| `answer` | The answer body — required, stored as sanitized safe HTML, search-indexed. |
| `keywords` | Optional search keywords (search-indexed; no dedicated edit-form field in this version — see KL-050.3). |
| `ispublished` | Public-listing flag (1 = Public/published, 0 = Internal/private). |
| `notes` | Internal staff notes (not shown to the public). |
| `created` / `updated` | Creation and last-modification timestamps. |
| (derived) `attachments` | Count of joined attachment files. |
| (derived) `topics` | Associated help-topic ids/names. |

### FAQ Category (`faq_category` table, `Category` class — CRUD owned by FS-032)

| Field (functional) | Role |
|--------------------|------|
| `category_id` | Category identifier. |
| `name` | Category name — required, ≥ 3 chars, unique. |
| `description` | Required, stored as safe HTML, shown on listings/views and search-indexed. |
| `ispublic` | Public-visibility flag (1 = Public, 0 = Internal). |
| `notes` | Internal notes. |
| `created` / `updated` | Timestamps. |
| (derived) `faqs` | Count of articles in the category. |

### Associations & attachments

- **`faq_attachment`** — join rows linking an article to content-addressed files (file storage owned by FS-022/FS-091).
- **`faq_topic`** — many-to-many join linking articles to help topics (help-topic entity owned by FS-030).

### Configuration

- **`enable_kb`** (config key, owned by FS-032/FS-091) — boolean toggle for the public client KB interface; default off in the schema. Composed with `FAQ::countPublishedFAQs()` by `isKnowledgebaseEnabled()`.

### Permissions

- **`can_manage_faq`** (group permission flag, owned by FS-031) — grants FAQ create/edit/publish/delete; surfaced as `canManageFAQ()` on the staff session.

### Key entry points & views

| Surface | File | Role |
|---------|------|------|
| Public bootstrap/guard | `kb/kb.inc.php` | Loads client context; redirects when KB unavailable. |
| Public landing | `kb/index.php` → `include/client/knowledgebase.inc.php` | Category listing + search form/results. |
| Public article/category router | `kb/faq.php` | Routes to article / category / landing views. |
| Public category view | `include/client/faq-category.inc.php` | One public category's published articles. |
| Public article view | `include/client/faq.inc.php` | One published article. |
| Staff browser | `scp/kb.php` → `faq-categories.inc.php` / `faq-category.inc.php` | Category listing/search/detail. |
| Staff article router + mutations | `scp/faq.php` | View/edit/publish/unpublish/delete dispatch. |
| Staff article view | `include/staff/faq-view.inc.php` | Single-article management view. |
| Staff create/edit form | `include/staff/faq.inc.php` | Article form (manager-only). |
| Inline preview AJAX | `include/ajax.kbase.php` (`KbaseAjaxAPI::faq`) | `ajax.php/kb/faq/<id>` hover snippet. |
| Admin KB toggle | `include/staff/settings-kb.inc.php` | `enable_kb` checkbox (page owned by FS-032). |

---

## User Flows / Interactions

### Flow 1: Public visitor browses and reads a FAQ
1. Visitor opens the support center; the "Knowledgebase" link appears only if the KB is enabled and has published public content.
2. Visitor clicks it → `kb/index.php`; the guard passes and the landing page lists public categories (with counts) and a search form.
3. Visitor clicks a category → `faq.php?cid=<id>` → the category view lists the category's published articles.
4. Visitor clicks an article → `faq.php?id=<id>` → the single-article view shows the question, answer, attachments, help topics, and breadcrumbs.

### Flow 2: Public visitor searches
1. On the landing page the visitor types a query, optionally picks a category and/or help-topic filter, and submits.
2. The form GETs `index.php?a=search&q=...&cid=...&topicId=...`.
3. The search returns a numbered list of matching published, public articles (or "The search did not match any FAQs.").
4. Hovering a result shows the inline preview snippet (FS-050.10).

### Flow 3: Manager creates and publishes a FAQ
1. Manager opens the staff KB tab → `kb.php`, browses categories (public and internal), clicks "Add New FAQ" within a category (or via the article form directly).
2. The form (`faq.inc.php`) collects question, category, listing type, answer, attachments, help topics, notes.
3. On submit (`do=create`) the article is validated and inserted; topics and attachments are linked; "FAQ added successfully" is shown.
4. From the article view, the manager uses the Options panel to **Publish** the article; if its category is public, it becomes visible to the public.

### Flow 4: Manager edits / unpublishes / deletes a FAQ
1. From the article view, the manager picks an Options action (Edit / Publish / Unpublish / Delete) and submits (`do=manage-faq`).
2. Edit opens the pre-filled form; saving updates the article, re-syncs topics, and applies attachment keep/delete; "FAQ updated successfully".
3. Unpublish hides the article from the public; Delete removes the article, its attachments, and topic links.

### Flow 5: Admin enables/disables the public KB
1. Admin opens Settings → Knowledge Base (`settings.php?t=kb`).
2. Admin toggles "Enable Knowledge base (Client interface)" and saves.
3. With it off (or with no published public FAQs), the public KB link disappears and entry scripts redirect; staff FAQ management is unaffected.

---

## Edge Cases & Error Scenarios

### EC-050.1: Unknown or invalid FAQ / category id
**Scenario**: A request supplies an `id`/`cid` that does not resolve.
**Expected**: The script records "Unknown or invalid FAQ" / "Unknown or invalid FAQ category" and falls back to the landing/categories view; no article/category content is shown.

### EC-050.2: Requesting an internal article publicly
**Scenario**: A public visitor requests `faq.php?id=<x>` where the article is unpublished or its category is internal.
**Expected**: `isPublished()` is false, so the single-article view is not selected; the request falls through to the landing page, and the article partial's own `die('Access Denied')` guard would block direct inclusion. The article never renders publicly.

### EC-050.3: KB disabled or empty mid-session
**Scenario**: An admin disables `enable_kb` (or the last published public FAQ is unpublished/deleted) while a visitor is browsing.
**Expected**: The next public KB request fails the guard and redirects to `../`; the navigation link no longer appears.

### EC-050.4: Non-manager attempts a management action
**Scenario**: A staff member without `canManageFAQ()` reaches an edit/add URL or posts a mutation.
**Expected**: The edit/add form `die('Access Denied')`s; the view-selection logic only switches to the form when `canManageFAQ()` is true; management affordances are not rendered. (See KL-050.1 for the limits of this gating on POST handlers.)

### EC-050.5: Duplicate question on create/edit
**Scenario**: A manager submits a question already used by another article.
**Expected**: Validation rejects with "Question already exists"; the form re-renders with POSTed values and the inline error.

### EC-050.6: Editing attachments — unchecking to delete
**Scenario**: A manager unchecks an existing attachment and saves.
**Expected**: The unchecked attachment is detached; if the underlying file becomes orphaned it is reclaimed. Newly selected files are uploaded and linked.

### EC-050.7: Category with zero (published) articles
**Scenario**: A public category has no published articles, or a staff category has no articles.
**Expected**: The public category view prints "Category does not have any FAQs." (with back-link); the staff view prints "Category does not have FAQs". The public landing listing omits categories with zero published articles entirely (`HAVING faqs>0`).

### EC-050.8: Article with no help topics
**Scenario**: An article has no associated help topics.
**Expected**: The "Help Topics:" line renders blank (a single space); the help-topic filter never surfaces such an article.

### EC-050.9: Search with empty result set
**Scenario**: A query matches nothing.
**Expected**: "The search did not match any FAQs." is shown (both public and staff).

### EC-050.10: Non-existent numeric category id on the public/staff router
**Scenario**: A request supplies a `cid` that is numeric but references no category row (`faq.php?cid=99999`).
**Expected**: `Category::lookup` still returns a hollow `Category` object (it never short-circuits on a missing row for numeric input), so the "Unknown or invalid FAQ category" error is **not** recorded. The hollow category's `isPublic()` is falsy, so the category view is not selected and the request falls through to the landing/categories page. The category-view partial would `die('Access Denied')` if it were reached with a non-public/hollow category. See KL-050.8.

### EC-050.11: Successful create lands on the new article's view
**Scenario**: A manager submits `do=create` and the article saves successfully.
**Expected**: The new-article object is assigned to `$faq`, so the post-POST view is the single-article management view (`faq-view.inc.php`) for the freshly created article — not the (now-cleared) form — and "FAQ added successfully" is shown.

### EC-050.12: Mismatched hidden id on edit submit
**Scenario**: An edit POST carries a hidden `id` that differs from the article id being saved.
**Expected**: `FAQ::save` sets **"Internal error. Try again"** and rejects the save (defensive id-tamper guard); no fields are written.

### EC-050.13: Category delete returns an undefined count on a no-op delete
**Scenario**: `Category::delete` is invoked (from FS-032) for a category whose row no longer exists or whose DELETE affects zero rows.
**Expected**: The `$num` return value is only assigned inside the `if(affected_rows)` branch, so a zero-affect delete returns an undefined/null value (and skips the cascade DELETE of its FAQ rows). Callers treating the return as a truthy success indicator see a falsy result. (Behavioral quirk noted; category delete is owned by FS-032.)

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Client request bootstrap (`client.inc.php`, `$cfg`, `$thisclient`, `UserNav`) | FS-001 | Public KB scripts boot through it; the KB nav link is added by `UserNav`. |
| Staff request bootstrap (`staff.inc.php`, `$thisstaff`, `StaffNav`, `kbase` tab) | FS-001 / FS-002 | Staff KB scripts boot through it; the `kbase` sub-nav is assembled here. |
| CSRF token enforcement (`csrf_token()`) | FS-002 | All staff FAQ mutation forms carry and require it. |
| Group permission `can_manage_faq` / `canManageFAQ()` | FS-031 | Gates all FAQ management. |
| FAQ **category** CRUD (`scp/categories.php`, `categor*.inc.php`, `Category` create/edit/delete UI) | FS-032 | This spec consumes `Category` as the parent grouping; category management UI is owned there. |
| KB enable toggle persistence (`enable_kb` on the settings page) | FS-032 / FS-033 | This spec consumes the toggle's effect; the settings page owns its persistence. |
| Help topics (entity, `parent / child` path, `ispublic` flag) | FS-030 | Articles associate to help topics; topic entities are defined there. |
| File storage / attachments (`AttachmentFile`, `file.php?h=`, content-addressed chunks, `deleteOrphans`) | FS-022 / FS-091 | FAQ attachments reference shared file storage. |
| Format/sanitization helpers (`Format::safe_html`, `striptags`, `htmlchars`, date formatters) | FS-003 | Used to sanitize questions/answers/descriptions and format dates. |
| AJAX dispatcher (`ajax.php`, `AjaxController`, `kb` namespace) | FS-043 | Routes `ajax.php/kb/faq/<id>` to `KbaseAjaxAPI`. |
| Canned responses (the co-located `cannedResp` AJAX method; the "Canned Responses" sub-tab) | FS-022 | Shares the `kbase` tab and the `KbaseAjaxAPI` controller; canned-response behavior owned there. |
| Reference data — `faq`, `faq_category`, `faq_attachment`, `faq_topic` schemas; `enable_kb` config key | FS-091 | Canonical table/column/config definitions. |

---

## Known Limitations

### KL-050.1: Management Gating on POST Handlers Is Partly View-Side
**Limitation**: The publish/unpublish/delete/update branches of `scp/faq.php` dispatch on `do`/`a` before the manager check; the strong gate is on the edit/add **view** (`die('Access Denied')`) and on the affordances that link to those actions, plus `canManageFAQ()` guarding the switch to the edit form. The mutation handlers themselves rely on the surrounding nav/permission context rather than an explicit per-action `canManageFAQ()` check.
**Impact**: A production hardening would add an explicit capability check at the top of each mutation branch.

### KL-050.2: Public Article "Last Updated" Reads the Category Timestamp
**Limitation**: The public single-article view (`faq.inc.php`) displays the **parent category's** `updated` date as the article's "Last updated", not the article's own timestamp. The staff view uses the article's own timestamp.
**Impact**: The public "Last updated" date can be misleading (it changes when the category is edited, not the article).

### KL-050.3: Keywords Are Search-Indexed but Have No Edit-Form Field
**Limitation**: The `keywords` column is matched by search (`FAQ::setKeywords` exists and search queries `keywords`), but the create/edit form exposes no keywords input in this version. Keywords therefore remain empty unless set out-of-band.
**Impact**: The keywords search dimension is effectively inert through the UI.

### KL-050.4: Category-Delete Cascade Skips the FAQ Topic-Cleanup Path
**Limitation**: `Category::delete` deletes the category row and its FAQ rows directly via SQL, bypassing `FAQ::delete` (which is what removes `faq_topic` and attachment join rows). Deleting a category can therefore leave orphaned `faq_topic` (and potentially attachment) rows for the deleted articles.
**Impact**: Dangling association rows accumulate; orphaned attachment files may not be reclaimed on category delete.

### KL-050.5: `Knowledgebase` Class Is Dead/Legacy Code Bound to the Canned Table
**Limitation**: `include/class.knowledgebase.php` defines a `Knowledgebase` class whose queries operate entirely on the canned-response table (`CANNED_TABLE`, `canned_id`), not on any FAQ table, and no entry script in the KB subsystem instantiates it. Its `validate`/`save` paths contain incomplete TODOs and at least one malformed SQL (a missing closing paren in `create`).
**Impact**: The class is non-functional dead code; the live staff KB is the FAQ subsystem. It should be removed or reconciled. Canned responses proper are owned by FS-022.

### KL-050.6: No Article Versioning, Ordering, or Rich Categorization
**Limitation**: Articles have no version history, no manual sort order (lists are alphabetized by question/name), no nesting of categories, and a single category per article.
**Impact**: Limited curation; large knowledge bases cannot be hand-ordered or hierarchically organized.

### KL-050.7: Search Is Unindexed SQL `LIKE` Substring Matching
**Limitation**: KB search is implemented as `LIKE '%term%'` across columns with no full-text index, ranking, stemming, or multi-term parsing (the whole query string is one substring).
**Impact**: Multi-word queries match only as a literal contiguous substring; performance and relevance degrade on large datasets.

### KL-050.8: `Category::lookup` Does Not Validate Row Existence
**Limitation**: `Category::lookup($id)` returns a `Category` object for **any** numeric id (its test is `is_numeric($id) && (new Category($id))`), even when no row exists — unlike `FAQ::lookup`, which additionally verifies `getId()==$id`. A hollow category (empty hashtable, `getId()==0`, falsy `isPublic()`/`getName()`) is therefore handed back instead of null for a bad numeric `cid`.
**Impact**: The "Unknown or invalid FAQ category" error path on the public and staff routers is effectively dead for numeric input; downstream code relies on the secondary `isPublic()`/access-denied guards to avoid rendering a non-existent category. Non-numeric `cid` does return null.

### KL-050.9: Public Search Result Carries an Inert Published/Internal Annotation
**Limitation**: The public search result template (`knowledgebase.inc.php`) passes a `Published`/`Internal` argument derived from `$row['ispublished']`, but the public search SQL never selects the `ispublished` column, so the argument is always empty and unused (public results are all published anyway).
**Impact**: Cosmetic/dead artifact; no functional effect. The staff search and listing views *do* select and render the Published/Internal annotation.

### KL-050.10: FAQ Notes Are Stored Without Sanitization
**Limitation**: `FAQ::save` stores the internal `notes` field verbatim (no `Format::safe_html`, no tag-stripping), unlike the answer (sanitized) and question (tag-stripped). Notes are never rendered to the public, but any internal surface that echoes notes without escaping would render their raw content.
**Impact**: Internal-notes content is trusted as-is in storage; consumers must escape on output. (No notes-rendering surface exists in the current FAQ views.)

### KL-050.11: `getHelpTopics` / `getHelpTopicsIds` Are Cached Once Per Object
**Limitation**: `getHelpTopics()` memoizes its result in `$this->topics` and `getHelpTopicsIds()` memoizes in `$this->ht['topics']`; neither is invalidated by `updateTopics()`. Within a single request that both mutates and re-reads topics on the same object instance, stale topic lists can be returned until the object is reloaded.
**Impact**: In practice the mutation paths `reload()` the object (or operate on fresh lookups), masking the staleness; it is a latent caching hazard rather than an observed user-facing bug.

---

## Future Considerations

- Add an explicit `canManageFAQ()` check at the head of each `scp/faq.php` mutation branch (KL-050.1).
- Use the article's own `updated` timestamp on the public view (KL-050.2).
- Expose a keywords field on the FAQ form, or remove the inert keywords search dimension (KL-050.3).
- Route category deletion through `FAQ::delete` (or add cascade cleanup) to avoid orphaned association/attachment rows (KL-050.4).
- Remove or reconcile the dead `Knowledgebase` class (KL-050.5).
- Article versioning, manual ordering, nested categories, and multi-category membership (KL-050.6).
- Replace `LIKE` search with a full-text index and multi-term/ranked search (KL-050.7).
- Make `Category::lookup` validate row existence (return null for a missing numeric id) for parity with `FAQ::lookup` (KL-050.8).
- Remove the inert published/internal annotation from the public search template, or select the column (KL-050.9).
- Decide whether internal notes should be sanitized on save or escaped on every output surface (KL-050.10).
- Invalidate cached topic lists in `updateTopics`, or always reload before re-reading (KL-050.11).
