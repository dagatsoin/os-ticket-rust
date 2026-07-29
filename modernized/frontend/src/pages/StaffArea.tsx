// Staff area (TS-M1-C4): login, the Open-tickets queue, and the single-ticket
// detail with a reply-only box. The staff auth store holds the /api/staff/me
// PROFILE (never a token); a 401 from any staff route is turned into a redirect
// to /staff/login by the apiClient (see TS-M1-A5).
//
// @implements BS-002: staff login + authenticated profile.
// @implements BS-020: open-tickets queue (number/subject/email/created).
// @implements BS-021: ticket detail (full thread) + reply-then-refresh.
// @implements FS-020.2: queue tabs with counts (TS-M3-A4).
// @implements FS-020.6: pagination (TS-M3-B4).
// @implements FS-020.7: sortable column headers (TS-M3-B4).
// @implements FS-020.3: overdue badge display (TS-M3-D5).
// @implements TS-M3-C4: workflow action buttons + dialogs.
// @implements TS-M3-E3: note form UI + thread display of notes.
// @implements TS-M3-F3: search box + advanced search dialog UI.
// @implements TS-M3-G2: checkboxes + bulk action bar UI.
// @implements TS-M3-I4: edit form UI.
// @implements TS-M3-I5: delete confirmation dialog.
// @implements TS-M3-I6: lock UI (warning banner + auto-renew polling).
import { useCallback, useEffect, useState } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Divider,
  FormControl,
  FormHelperText,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import {
  Link as RouterLink,
  Navigate,
  useNavigate,
  useParams,
  useSearchParams,
} from "react-router-dom";
import { useStores } from "../stores/StoreContext";
import { CredentialForm, type CredentialField } from "../components/CredentialForm";
import { ThreadView, type ThreadEntry } from "../components/ThreadView";
import { AttachmentList } from "../components/AttachmentList";
import { AttachmentChip } from "../components/AttachmentChip";
import { AnsweredBadge } from "../components/AnsweredBadge";
import { QueueTabs } from "../components/QueueTabs";
import { SortableHeader } from "../components/SortableHeader";
import { Pagination } from "../components/Pagination";
import { OverdueBadge } from "../components/OverdueBadge";
import { WorkflowToolbar } from "../components/WorkflowToolbar";
import { NoteForm } from "../components/NoteForm";
import { SearchBox } from "../components/SearchBox";
import { AdvancedSearchDialog, type AdvancedSearchFilters } from "../components/AdvancedSearchDialog";
import { BulkActionBar } from "../components/BulkActionBar";
import { TicketEditForm } from "../components/TicketEditForm";
import { DeleteConfirmDialog } from "../components/DeleteConfirmDialog";
import { LockWarningBanner } from "../components/LockWarningBanner";
import { useLock } from "../hooks/useLock";
import {
  ALLOWED_EXTENSIONS,
  ALLOWED_EXTENSIONS_LABEL,
  MAX_FILE_SIZE_LABEL,
} from "../utils/validateAttachment";
import { formatTimestamp } from "../utils/formatTimestamp";
import type {
  StaffThreadEntry,
  QueueStatus,
  SortKey,
  SortOrder,
  RightmostColumn,
  DepartmentOption,
} from "../stores/StaffTicketStore";

/** `accept` for the reply own-file input (seeded allow-list). */
const REPLY_FILE_ACCEPT = ALLOWED_EXTENSIONS.join(",");

const STAFF_LOGIN_FIELDS: CredentialField[] = [
  { name: "username", label: "Username", required: true, autoComplete: "username" },
  { name: "password", label: "Password", type: "password", required: true, autoComplete: "current-password" },
];

/**
 * Map a staff thread entry to the realm-agnostic ThreadView shape, composing the
 * body text with clickable attachment chips (B2). Staff downloads are
 * ticket-scoped, so the AttachmentList needs the ticket id.
 */
function toThreadEntry(e: StaffThreadEntry, ticketId: number): ThreadEntry {
  const kind = e.threadType === "R" ? "response" : e.threadType === "N" ? "note" : "message";
  const author = e.threadType === "N" ? `${e.poster} (internal note)` : e.poster;
  return {
    id: e.id,
    author,
    // The staff detail thread entries (ThreadEntryView in backend
    // api/src/staff.rs) now carry a per-entry `created` (RFC3339); render it
    // as a locale date-time via the shared formatter.
    timestamp: formatTimestamp(e.created),
    title: e.title,
    kind,
    body: (
      <>
        <Typography variant="body2" sx={{ whiteSpace: "pre-wrap" }}>
          {e.body}
        </Typography>
        <AttachmentList realm="staff" ticketId={ticketId} attachments={e.attachments} />
      </>
    ),
  };
}

/** `/staff` index — route authenticated agents to the queue, else to login. */
export const StaffDashboardPage = observer(function StaffDashboardPage() {
  const { staffAuth } = useStores();
  return <Navigate to={staffAuth.isAuthenticated ? "/staff/tickets" : "/staff/login"} replace />;
});

/** Staff sign-in (CredentialForm; generic error on failure). */
export const StaffLoginPage = observer(function StaffLoginPage() {
  const { staffAuth } = useStores();
  const navigate = useNavigate();

  if (staffAuth.isAuthenticated) {
    return <Navigate to="/staff/tickets" replace />;
  }

  return (
    <Stack spacing={3} sx={{ maxWidth: 420 }}>
      <Typography variant="h5">Staff Sign In</Typography>
      <CredentialForm
        fields={STAFF_LOGIN_FIELDS}
        submitLabel="Sign In"
        onSubmit={async (values) => {
          await staffAuth.login(values);
          navigate("/staff/tickets");
        }}
      />
    </Stack>
  );
});

/** Map rightmost column type to header label. */
const RIGHTMOST_COLUMN_LABELS: Record<RightmostColumn, string> = {
  assigned_to: "Assigned To",
  closed_by: "Closed By",
  department: "Department",
};

/** Map rightmost column type to sort key. */
const RIGHTMOST_COLUMN_SORT_KEY: Record<RightmostColumn, SortKey> = {
  assigned_to: "assignee",
  closed_by: "staff",
  department: "dept",
};

/**
 * Queue page with tabs, sortable headers, pagination, search, and bulk actions.
 * @implements FS-020.2: queue tabs with counts
 * @implements FS-020.6: pagination
 * @implements FS-020.7: sortable column headers
 * @implements FS-020.3: overdue badge display
 * @implements TS-M3-F3: search box + advanced search dialog
 * @implements TS-M3-G2: checkboxes + bulk action bar
 */
export const StaffQueuePage = observer(function StaffQueuePage() {
  const { staffTickets, staffAuth, ticketOptions } = useStores();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  // Advanced search dialog state
  const [advancedSearchOpen, setAdvancedSearchOpen] = useState(false);

  // Extract URL params.
  const status = (searchParams.get("status") as QueueStatus | null) ?? "open";
  const sortParam = searchParams.get("sort") as SortKey | null;
  const orderParam = searchParams.get("order") as SortOrder | null;
  const pageParam = searchParams.get("p");
  const limitParam = searchParams.get("limit");
  const searchAction = searchParams.get("a");
  const searchQuery = searchParams.get("query") ?? "";

  const page = pageParam ? parseInt(pageParam, 10) : 1;
  const limit = limitParam ? parseInt(limitParam, 10) : undefined;

  const isSearchMode = searchAction === "search";

  // Extract permissions from auth profile
  const canCloseTickets = (staffAuth.user as Record<string, unknown>)?.canCloseTickets !== false;
  const canDeleteTickets = (staffAuth.user as Record<string, unknown>)?.canDeleteTickets === true;
  const canManageTickets = canCloseTickets || canDeleteTickets;

  // Advanced-search filter options come from the shared ticket-options store
  // (GET /api/staff/ticket-options) — real departments / agents / help topics,
  // no longer hardcoded literal DB ids. Advanced search filters by numeric id, so
  // the raw `agents` list (numeric ids) is used here — not the prefixed combined
  // assignee list the Assign dialog needs.
  const searchDepartments = ticketOptions.departments;
  const searchAssignees = ticketOptions.agents;
  const searchTopics = ticketOptions.helpTopics;

  // Load stats + reference options on mount.
  useEffect(() => {
    void staffTickets.loadStats();
    void staffTickets.loadSortPrefs();
    void ticketOptions.load();
  }, [staffTickets, ticketOptions]);

  // Reload queue when URL params change.
  // IMPORTANT: When no explicit sort/order params are in the URL, we must wait for
  // sortPrefs to load first to correctly apply sticky sort preferences (BS-020.8).
  useEffect(() => {
    if (isSearchMode) {
      // Search mode
      void staffTickets.loadSearchResults({
        query: searchQuery,
        status: searchParams.get("status") ?? undefined,
        deptId: searchParams.get("dept_id") ? parseInt(searchParams.get("dept_id")!, 10) : undefined,
        assigneeId: searchParams.get("assignee_id") ? parseInt(searchParams.get("assignee_id")!, 10) : undefined,
        topicId: searchParams.get("topic_id") ? parseInt(searchParams.get("topic_id")!, 10) : undefined,
        startDate: searchParams.get("start_date") ?? undefined,
        endDate: searchParams.get("end_date") ?? undefined,
        page,
        limit,
        sort: sortParam ?? undefined,
        order: orderParam ?? undefined,
      });
    } else {
      // Normal queue mode - ensure sortPrefs loaded before applying sticky sort
      const loadQueueWithStickySort = async () => {
        // If no explicit sort in URL, wait for sortPrefs to be loaded first
        if (!sortParam) {
          await staffTickets.ensureSortPrefsLoaded();
        }
        const stickyPref = staffTickets.sortPrefs[status];
        const sort = sortParam ?? stickyPref?.sort ?? undefined;
        const order = orderParam ?? stickyPref?.order ?? undefined;

        staffTickets.setSearchMode(false);
        void staffTickets.loadQueue({
          status,
          sort,
          order,
          page,
          limit,
          refreshStats: false,
        });
      };
      void loadQueueWithStickySort();
    }
  }, [staffTickets, status, sortParam, orderParam, page, limit, isSearchMode, searchQuery, searchParams]);

  // Tab change handler: update URL status param, reset page to 1.
  const handleTabChange = useCallback(
    (newStatus: QueueStatus) => {
      const newParams = new URLSearchParams();
      if (newStatus !== "open") {
        newParams.set("status", newStatus);
      }
      // Clear search and pagination on tab switch
      setSearchParams(newParams);
    },
    [setSearchParams],
  );

  // Sort handler: clicking a column header.
  const handleSort = useCallback(
    (sortKey: SortKey) => {
      const newParams = new URLSearchParams(searchParams);
      const isActive = staffTickets.currentSort === sortKey;
      const newOrder: SortOrder =
        isActive && staffTickets.currentOrder === "DESC" ? "ASC" : "DESC";

      newParams.set("sort", sortKey);
      newParams.set("order", newOrder);
      // Reset to page 1 on sort change.
      newParams.delete("p");
      setSearchParams(newParams);
    },
    [searchParams, setSearchParams, staffTickets.currentSort, staffTickets.currentOrder],
  );

  // Page change handler.
  const handlePageChange = useCallback(
    (newPage: number) => {
      const newParams = new URLSearchParams(searchParams);
      if (newPage === 1) {
        newParams.delete("p");
      } else {
        newParams.set("p", String(newPage));
      }
      setSearchParams(newParams);
    },
    [searchParams, setSearchParams],
  );

  // Quick search handler
  const handleQuickSearch = useCallback(
    (query: string) => {
      const newParams = new URLSearchParams();
      newParams.set("a", "search");
      newParams.set("query", query);
      setSearchParams(newParams);
    },
    [setSearchParams],
  );

  // Advanced search handler
  const handleAdvancedSearch = useCallback(
    (filters: AdvancedSearchFilters) => {
      const newParams = new URLSearchParams();
      newParams.set("a", "search");
      if (filters.keyword) newParams.set("query", filters.keyword);
      if (filters.status && filters.status !== "any") newParams.set("status", filters.status);
      if (filters.deptId) newParams.set("dept_id", String(filters.deptId));
      if (filters.assigneeId) newParams.set("assignee_id", String(filters.assigneeId));
      if (filters.topicId) newParams.set("topic_id", String(filters.topicId));
      if (filters.startDate) newParams.set("start_date", filters.startDate);
      if (filters.endDate) newParams.set("end_date", filters.endDate);
      setSearchParams(newParams);
    },
    [setSearchParams],
  );

  // Clear search handler
  const handleClearSearch = useCallback(() => {
    setSearchParams(new URLSearchParams());
  }, [setSearchParams]);

  // Bulk action complete handler
  const handleBulkActionComplete = useCallback(() => {
    // Reload queue and stats after bulk action
    void staffTickets.loadStats();
    if (isSearchMode) {
      void staffTickets.loadSearchResults({
        query: searchQuery,
        page,
        limit,
      });
    } else {
      void staffTickets.loadQueue({ status, page, limit, refreshStats: true });
    }
  }, [staffTickets, isSearchMode, searchQuery, status, page, limit]);

  // Handle row checkbox click (stop propagation to prevent navigation)
  const handleCheckboxClick = useCallback(
    (e: React.MouseEvent, ticketId: number) => {
      e.stopPropagation();
      staffTickets.toggleTicketSelection(ticketId);
    },
    [staffTickets],
  );

  // Get the rightmost column configuration.
  const rightmostColumn = staffTickets.rightmostColumn;
  const rightmostLabel = RIGHTMOST_COLUMN_LABELS[rightmostColumn];
  const rightmostSortKey = RIGHTMOST_COLUMN_SORT_KEY[rightmostColumn];

  // Get the rightmost column value for a queue item.
  const getRightmostValue = (item: typeof staffTickets.queue[0]): string => {
    switch (rightmostColumn) {
      case "assigned_to":
        return item.assigned_to_name ?? "-";
      case "closed_by":
        return item.closed_by_name ?? "-";
      case "department":
        return item.department_name ?? "-";
    }
  };

  // Queue title based on status or search mode
  const queueTitle = isSearchMode
    ? "Search Results"
    : status === "open"
      ? "Open Tickets"
      : status === "answered"
        ? "Answered Tickets"
        : status === "assigned"
          ? "My Tickets"
          : status === "overdue"
            ? "Overdue Tickets"
            : "Closed Tickets";

  // Show Status column instead of Priority when search results span multiple statuses
  const showStatusColumn = staffTickets.showStatusColumn;

  return (
    <Stack spacing={3}>
      <Stack direction="row" justifyContent="space-between" alignItems="center">
        <Typography variant="h5">
          {queueTitle}
          {isSearchMode && searchQuery && ` for "${searchQuery}"`}
        </Typography>
        <Button
          variant="text"
          onClick={async () => {
            await staffAuth.logout();
            navigate("/staff/login");
          }}
        >
          Log Out
        </Button>
      </Stack>

      {/* Search box (TS-M3-F3) */}
      <SearchBox
        query={searchQuery}
        onSearch={handleQuickSearch}
        onAdvancedOpen={() => setAdvancedSearchOpen(true)}
        onClear={handleClearSearch}
        isSearchActive={isSearchMode}
      />

      {/* Advanced search dialog */}
      <AdvancedSearchDialog
        open={advancedSearchOpen}
        onClose={() => setAdvancedSearchOpen(false)}
        onSearch={handleAdvancedSearch}
        initialKeyword={searchQuery}
        departments={searchDepartments}
        assignees={searchAssignees}
        topics={searchTopics}
      />

      {/* Queue tabs (TS-M3-A4) - hidden in search mode */}
      {!isSearchMode && (
        <QueueTabs
          activeStatus={status}
          stats={staffTickets.stats}
          onTabChange={handleTabChange}
        />
      )}

      {/* Bulk action bar (TS-M3-G2) */}
      {canManageTickets && (
        <BulkActionBar
          queueStatus={status}
          canCloseTickets={canCloseTickets}
          canDeleteTickets={canDeleteTickets}
          onActionComplete={handleBulkActionComplete}
        />
      )}

      {staffTickets.queueError ? (
        <Alert severity="error">{staffTickets.queueError}</Alert>
      ) : null}

      <Paper variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              {/* Checkbox column for bulk selection */}
              {canManageTickets && (
                <TableCell padding="checkbox">
                  <Checkbox
                    checked={staffTickets.allSelected}
                    indeterminate={staffTickets.someSelected}
                    onChange={() => {
                      if (staffTickets.allSelected) {
                        staffTickets.deselectAllTickets();
                      } else {
                        staffTickets.selectAllTickets();
                      }
                    }}
                    inputProps={{ "aria-label": "Select all tickets" }}
                    data-testid="header-select-all"
                  />
                </TableCell>
              )}
              <SortableHeader
                sortKey="ID"
                label="Ticket #"
                activeSort={staffTickets.currentSort}
                activeOrder={staffTickets.currentOrder}
                onSort={handleSort}
              />
              <SortableHeader
                sortKey="date"
                label="Date"
                activeSort={staffTickets.currentSort}
                activeOrder={staffTickets.currentOrder}
                onSort={handleSort}
              />
              <SortableHeader
                sortKey="subj"
                label="Subject"
                activeSort={staffTickets.currentSort}
                activeOrder={staffTickets.currentOrder}
                onSort={handleSort}
              />
              <SortableHeader
                sortKey="name"
                label="From"
                activeSort={staffTickets.currentSort}
                activeOrder={staffTickets.currentOrder}
                onSort={handleSort}
              />
              {/* Show Status instead of Priority when searching across statuses */}
              {showStatusColumn ? (
                <TableCell>Status</TableCell>
              ) : (
                <SortableHeader
                  sortKey="pri"
                  label="Priority"
                  activeSort={staffTickets.currentSort}
                  activeOrder={staffTickets.currentOrder}
                  onSort={handleSort}
                />
              )}
              {!showStatusColumn && <TableCell>Status</TableCell>}
              <SortableHeader
                sortKey={rightmostSortKey}
                label={rightmostLabel}
                activeSort={staffTickets.currentSort}
                activeOrder={staffTickets.currentOrder}
                onSort={handleSort}
              />
            </TableRow>
          </TableHead>
          <TableBody>
            {staffTickets.queue.length === 0 ? (
              <TableRow>
                <TableCell colSpan={canManageTickets ? 8 : 7}>
                  <Typography color="text.secondary">
                    {staffTickets.loadingQueue ? "Loading..." : "No tickets found."}
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              staffTickets.queue.map((t) => (
                <TableRow
                  key={t.id}
                  hover
                  sx={{ cursor: "pointer" }}
                  onClick={() => navigate(`/staff/tickets/${t.id}`)}
                  selected={staffTickets.selectedTicketIds.has(t.id)}
                  data-testid="queue-row"
                >
                  {/* Row checkbox */}
                  {canManageTickets && (
                    <TableCell padding="checkbox">
                      <Checkbox
                        checked={staffTickets.selectedTicketIds.has(t.id)}
                        onClick={(e) => handleCheckboxClick(e, t.id)}
                        inputProps={{ "aria-label": `Select ticket ${t.number}` }}
                        data-testid={`row-checkbox-${t.id}`}
                      />
                    </TableCell>
                  )}
                  <TableCell>{t.number}</TableCell>
                  <TableCell>{t.created}</TableCell>
                  <TableCell>
                    <Stack direction="row" spacing={1} alignItems="center">
                      {t.subject}
                      {/* Overdue icon in subject column (TS-M3-D5) */}
                      {t.isoverdue ? <OverdueBadge isoverdue compact duedate={t.duedate} /> : null}
                    </Stack>
                  </TableCell>
                  <TableCell>{t.email}</TableCell>
                  {showStatusColumn ? (
                    <TableCell>
                      <AnsweredBadge answered={t.isanswered} />
                    </TableCell>
                  ) : (
                    <>
                      <TableCell>{t.priority ?? "-"}</TableCell>
                      <TableCell>
                        <AnsweredBadge answered={t.isanswered} />
                      </TableCell>
                    </>
                  )}
                  <TableCell>{getRightmostValue(t)}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Paper>

      {/* Pagination (TS-M3-B4) */}
      {staffTickets.pagination ? (
        <Pagination pagination={staffTickets.pagination} onPageChange={handlePageChange} />
      ) : null}
    </Stack>
  );
});

/**
 * Single-ticket detail: full thread + reply/note tabs + workflow toolbar.
 * @implements TS-M3-C4: workflow action buttons (Claim/Assign/Transfer/Close/Reopen)
 * @implements TS-M3-E3: note form UI + thread display of notes
 * @implements TS-M3-I4: edit form UI
 * @implements TS-M3-I5: delete confirmation dialog
 * @implements TS-M3-I6: lock UI (warning banner + auto-renew polling)
 */
export const StaffTicketDetailPage = observer(function StaffTicketDetailPage() {
  const { staffTickets, staffAuth, ticketOptions } = useStores();
  const { id } = useParams();
  const ticketId = Number(id);

  // Tab state: "reply" or "note" (TS-M3-E3 AC-1)
  const [activeTab, setActiveTab] = useState<"reply" | "note">("reply");

  // Edit and delete dialog states (TS-M3-I4, TS-M3-I5)
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  // Lock management (TS-M3-I6)
  const { lockedByAnother, lockedByName, lockError } = useLock({
    ticketId,
    autoAcquire: true,
  });

  useEffect(() => {
    if (Number.isFinite(ticketId)) {
      void staffTickets.loadDetail(ticketId);
      void staffTickets.loadCanned(ticketId);
    }
    return () => staffTickets.clearDetail();
  }, [staffTickets, ticketId]);

  // Load the transfer/assign/edit reference lists once (cached across tickets).
  useEffect(() => {
    void ticketOptions.load();
  }, [ticketOptions]);

  const detail = staffTickets.detail;

  // Extract permissions from auth profile (default to true for development)
  const permissions = {
    canAssignTickets: (staffAuth.user as Record<string, unknown>)?.canAssignTickets !== false,
    canTransferTickets: (staffAuth.user as Record<string, unknown>)?.canTransferTickets !== false,
    canCloseTickets: (staffAuth.user as Record<string, unknown>)?.canCloseTickets !== false,
    canCreateTickets: (staffAuth.user as Record<string, unknown>)?.canCreateTickets !== false,
  };

  // Additional permissions for edit/delete
  const canEditTickets = (staffAuth.user as Record<string, unknown>)?.canEditTickets !== false;
  const canDeleteTickets = (staffAuth.user as Record<string, unknown>)?.canDeleteTickets === true;

  const currentStaffId = (staffAuth.user as Record<string, unknown>)?.id as number | undefined;

  // Real reference lists from GET /api/staff/ticket-options (no longer hardcoded
  // literal DB ids). Transfer → departments; Assign → combined staff+team
  // assignees (ids namespaced s<id>/t<id>, exactly what the assign route expects);
  // Edit properties → departments + help topics + priorities + SLA plans.
  const departments: DepartmentOption[] = ticketOptions.departments;
  const assignees = ticketOptions.assignees;
  const topics = ticketOptions.helpTopics;
  const priorities = ticketOptions.priorities;
  const slas = ticketOptions.slaPlans;

  async function handleReply() {
    if (!staffTickets.canReply) return;
    try {
      await staffTickets.reply(ticketId);
    } catch {
      // Error surfaced via staffTickets.replyError.
    }
  }

  function handleCannedChange(value: string) {
    if (value === "") {
      staffTickets.clearCanned();
      return;
    }
    void staffTickets.selectCanned(ticketId, Number(value));
  }

  // Determine if reply/note forms should be disabled due to lock
  const formsDisabled = lockedByAnother;

  // Reply form with lock-aware disable (existing M2 implementation, updated for lock)
  const replyFormWithLock = (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Stack spacing={2}>
        <Typography variant="subtitle2">Reply to customer</Typography>
        {staffTickets.replyError ? (
          <Alert severity="error">{staffTickets.replyError}</Alert>
        ) : null}

        {/* Canned-response dropdown */}
        <FormControl fullWidth size="small" disabled={staffTickets.cannedList.length === 0 || formsDisabled}>
          <InputLabel id="canned-response-label">Canned response</InputLabel>
          <Select
            labelId="canned-response-label"
            label="Canned response"
            value={staffTickets.selectedCannedId === null ? "" : String(staffTickets.selectedCannedId)}
            onChange={(e) => handleCannedChange(e.target.value)}
            data-testid="canned-select"
          >
            <MenuItem value="">
              <em>None</em>
            </MenuItem>
            {staffTickets.cannedList.map((c) => (
              <MenuItem key={c.id} value={String(c.id)}>
                {c.title}
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        <TextField
          label="Reply"
          multiline
          rows={4}
          fullWidth
          value={staffTickets.replyBody}
          onChange={(e) => staffTickets.setReplyBody(e.target.value)}
          disabled={formsDisabled}
          inputProps={{ "data-testid": "reply-textarea" }}
        />

        {/* Read-only chips carried by the selected canned response */}
        {staffTickets.cannedAttachments.length > 0 ? (
          <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
            {staffTickets.cannedAttachments.map((att) => (
              <AttachmentChip
                key={att.id}
                label={att.name}
                readOnlyMarker="from canned response"
              />
            ))}
          </Stack>
        ) : null}

        {/* Own-file input */}
        <Box>
          <Button
            variant="outlined"
            component="label"
            size="small"
            disabled={formsDisabled}
            data-testid="reply-file-button"
          >
            {staffTickets.replyFile ? "Change file" : "Attach a file (optional)"}
            <input
              type="file"
              hidden
              accept={REPLY_FILE_ACCEPT}
              data-testid="reply-file-input"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) staffTickets.setReplyFile(f);
                e.target.value = "";
              }}
            />
          </Button>
          {staffTickets.replyFile ? (
            <Box sx={{ mt: 1 }}>
              <AttachmentChip
                label={staffTickets.replyFile.name}
                onClick={() => staffTickets.clearReplyFile()}
              />
            </Box>
          ) : null}
          <FormHelperText error={Boolean(staffTickets.replyFileError)} data-testid="reply-file-helper">
            {staffTickets.replyFileError ??
              `Allowed types: ${ALLOWED_EXTENSIONS_LABEL}. Max size: ${MAX_FILE_SIZE_LABEL}.`}
          </FormHelperText>
        </Box>

        <Divider />
        <Box>
          <Tooltip title={formsDisabled ? "Ticket is locked by another agent" : ""}>
            <span>
              <Button
                variant="contained"
                onClick={() => void handleReply()}
                disabled={!staffTickets.canReply || formsDisabled}
                data-testid="reply-submit"
              >
                {staffTickets.replying ? "Sending..." : "Send Reply"}
              </Button>
            </span>
          </Tooltip>
        </Box>
      </Stack>
    </Paper>
  );

  // The slot that switches between Reply and Note forms (TS-M3-E3 AC-1)
  // Updated to use lock-aware reply form
  const composerSlotWithLock = (
    <Box data-testid="composer-section">
      {/* Tab buttons for Reply / Note */}
      <Stack direction="row" spacing={1} sx={{ mb: 2 }}>
        <Button
          variant={activeTab === "reply" ? "contained" : "outlined"}
          onClick={() => setActiveTab("reply")}
          data-testid="reply-tab"
        >
          Reply
        </Button>
        <Button
          variant={activeTab === "note" ? "contained" : "outlined"}
          onClick={() => setActiveTab("note")}
          data-testid="note-tab"
        >
          Note
        </Button>
      </Stack>

      {/* Show appropriate form based on active tab */}
      {activeTab === "reply" ? replyFormWithLock : (
        <NoteForm ticketId={ticketId} ticketStatus={detail?.status ?? "open"} />
      )}
    </Box>
  );

  return (
    <Stack spacing={3}>
      <Button component={RouterLink} to="/staff/tickets" variant="text" sx={{ alignSelf: "flex-start" }}>
        ← Back to queue
      </Button>

      {/* Lock warning banner (TS-M3-I6) */}
      <LockWarningBanner
        lockedByAnother={lockedByAnother}
        lockedByName={lockedByName}
        lockError={lockError}
      />

      {staffTickets.detailError ? (
        <Alert severity="error">{staffTickets.detailError}</Alert>
      ) : null}

      {detail ? (
        <>
          {/* Ticket header with status info */}
          <Box>
            <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap">
              <Typography variant="h5">{detail.subject}</Typography>
              <AnsweredBadge answered={detail.isanswered} />
              {/* Overdue status and due date (TS-M3-D5 AC-3, AC-4) */}
              {detail.isoverdue || detail.duedate ? (
                <OverdueBadge isoverdue={detail.isoverdue ?? false} duedate={detail.duedate} />
              ) : null}
            </Stack>
            <Typography color="text.secondary">
              Ticket #{detail.number} · {detail.name} · {detail.email}
              {detail.deptName && ` · ${detail.deptName}`}
            </Typography>
            {detail.closedByName && detail.status === "closed" && (
              <Typography color="text.secondary" variant="body2">
                Closed by: {detail.closedByName}
              </Typography>
            )}
          </Box>

          {/* Action buttons row: Workflow + Edit + Delete (TS-M3-I4, TS-M3-I5) */}
          <Stack direction="row" spacing={2} alignItems="center" flexWrap="wrap">
            {/* Workflow action toolbar (TS-M3-C4) */}
            {/* Key forces remount when ticket changes to reset dialog states */}
            <WorkflowToolbar
              key={`workflow-${detail.id}`}
              ticket={detail}
              permissions={permissions}
              currentStaffId={currentStaffId}
              departments={departments}
              assignees={assignees}
            />

            {/* Edit button (TS-M3-I4) */}
            {canEditTickets && (
              <Button
                variant="outlined"
                onClick={() => setEditDialogOpen(true)}
                data-testid="edit-button"
              >
                Edit
              </Button>
            )}

            {/* Delete button (TS-M3-I5) */}
            {canDeleteTickets && (
              <Button
                variant="outlined"
                color="error"
                onClick={() => setDeleteDialogOpen(true)}
                data-testid="delete-button"
              >
                Delete
              </Button>
            )}
          </Stack>

          {/* Thread view with reply/note composer slot */}
          <ThreadView
            entries={detail.entries.map((e) => toThreadEntry(e, ticketId))}
            replySlot={composerSlotWithLock}
          />

          {/* Edit form dialog (TS-M3-I4) */}
          <TicketEditForm
            open={editDialogOpen}
            onClose={() => setEditDialogOpen(false)}
            ticket={detail}
            onSuccess={() => void staffTickets.loadDetail(ticketId)}
            departments={departments}
            topics={topics}
            priorities={priorities}
            slas={slas}
          />

          {/* Delete confirmation dialog (TS-M3-I5) */}
          <DeleteConfirmDialog
            open={deleteDialogOpen}
            onClose={() => setDeleteDialogOpen(false)}
            ticketId={detail.id}
            ticketNumber={detail.number}
          />
        </>
      ) : (
        <Typography color="text.secondary">
          {staffTickets.loadingDetail ? "Loading..." : "Ticket not found."}
        </Typography>
      )}
    </Stack>
  );
});
