/**
 * Advanced search dialog component.
 * Multi-criteria search with keyword, status, department, assignee, topic, and date range.
 *
 * @implements TS-M3-F3: advanced search dialog UI
 */
import { useState, useEffect } from "react";
import { observer } from "mobx-react-lite";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  TextField,
} from "@mui/material";
import { useStores } from "../stores/StoreContext";

/** Department option for dropdown. */
export interface DeptOption {
  id: number;
  name: string;
}

/** Staff option for assignee dropdown. */
export interface StaffOption {
  id: number;
  name: string;
}

/** Help topic option for dropdown. */
export interface TopicOption {
  id: number;
  name: string;
}

export interface AdvancedSearchDialogProps {
  /** Whether dialog is open. */
  open: boolean;
  /** Called to close the dialog. */
  onClose: () => void;
  /** Called when search is submitted. */
  onSearch: (filters: AdvancedSearchFilters) => void;
  /** Initial keyword value. */
  initialKeyword?: string;
  /** Initial filters. */
  initialFilters?: Partial<AdvancedSearchFilters>;
  /** Available departments (only those the agent has access to). */
  departments: DeptOption[];
  /** Available assignees. */
  assignees: StaffOption[];
  /** Available help topics. */
  topics: TopicOption[];
}

/** Advanced search filter values. */
export interface AdvancedSearchFilters {
  keyword: string;
  status: string;
  deptId: number | "";
  assigneeId: number | "";
  topicId: number | "";
  startDate: string;
  endDate: string;
}

const STATUS_OPTIONS = [
  { value: "any", label: "Any" },
  { value: "open", label: "Open" },
  { value: "answered", label: "Answered" },
  { value: "overdue", label: "Overdue" },
  { value: "closed", label: "Closed" },
];

/**
 * Advanced search dialog with multi-criteria filters.
 * @implements TS-M3-F3 AC-4: Dialog with filter fields
 * @implements TS-M3-F3 AC-5: Department dropdown shows only accessible depts
 * @implements TS-M3-F3 AC-6: Filters combine correctly
 * @implements TS-M3-F3 AC-9: Date range pickers
 */
export const AdvancedSearchDialog = observer(function AdvancedSearchDialog({
  open,
  onClose,
  onSearch,
  initialKeyword = "",
  initialFilters = {},
  departments,
  assignees,
  topics,
}: AdvancedSearchDialogProps) {
  const { staffTickets } = useStores();

  const [keyword, setKeyword] = useState(initialKeyword);
  const [status, setStatus] = useState(initialFilters.status ?? "any");
  const [deptId, setDeptId] = useState<number | "">(initialFilters.deptId ?? "");
  const [assigneeId, setAssigneeId] = useState<number | "">(initialFilters.assigneeId ?? "");
  const [topicId, setTopicId] = useState<number | "">(initialFilters.topicId ?? "");
  const [startDate, setStartDate] = useState(initialFilters.startDate ?? "");
  const [endDate, setEndDate] = useState(initialFilters.endDate ?? "");

  // Reset form when dialog opens.
  useEffect(() => {
    if (open) {
      setKeyword(initialKeyword);
      setStatus(initialFilters.status ?? "any");
      setDeptId(initialFilters.deptId ?? "");
      setAssigneeId(initialFilters.assigneeId ?? "");
      setTopicId(initialFilters.topicId ?? "");
      setStartDate(initialFilters.startDate ?? "");
      setEndDate(initialFilters.endDate ?? "");
    }
  }, [open, initialKeyword, initialFilters]);

  const handleSubmit = () => {
    // Validate keyword if provided.
    if (keyword.trim() && !staffTickets.validateSearchQuery(keyword.trim())) {
      return;
    }
    onSearch({
      keyword: keyword.trim(),
      status,
      deptId,
      assigneeId,
      topicId,
      startDate,
      endDate,
    });
    onClose();
  };

  const handleReset = () => {
    setKeyword("");
    setStatus("any");
    setDeptId("");
    setAssigneeId("");
    setTopicId("");
    setStartDate("");
    setEndDate("");
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
      data-testid="advanced-search-dialog"
    >
      <DialogTitle>Advanced Search</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          {/* Keyword */}
          <TextField
            label="Keyword"
            fullWidth
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            error={Boolean(staffTickets.searchError)}
            helperText={staffTickets.searchError}
            inputProps={{ "data-testid": "advanced-keyword" }}
          />

          {/* Status */}
          <FormControl fullWidth>
            <InputLabel id="status-label">Status</InputLabel>
            <Select
              labelId="status-label"
              label="Status"
              value={status}
              onChange={(e) => setStatus(e.target.value)}
              data-testid="advanced-status"
            >
              {STATUS_OPTIONS.map((opt) => (
                <MenuItem key={opt.value} value={opt.value}>
                  {opt.label}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {/* Department */}
          <FormControl fullWidth>
            <InputLabel id="dept-label">Department</InputLabel>
            <Select
              labelId="dept-label"
              label="Department"
              value={deptId}
              onChange={(e) => setDeptId(e.target.value as number | "")}
              data-testid="advanced-department"
            >
              <MenuItem value="">
                <em>Any</em>
              </MenuItem>
              {departments.map((dept) => (
                <MenuItem key={dept.id} value={dept.id}>
                  {dept.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {/* Assignee */}
          <FormControl fullWidth>
            <InputLabel id="assignee-label">Assignee</InputLabel>
            <Select
              labelId="assignee-label"
              label="Assignee"
              value={assigneeId}
              onChange={(e) => setAssigneeId(e.target.value as number | "")}
              data-testid="advanced-assignee"
            >
              <MenuItem value="">
                <em>Any</em>
              </MenuItem>
              {assignees.map((staff) => (
                <MenuItem key={staff.id} value={staff.id}>
                  {staff.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {/* Help Topic */}
          <FormControl fullWidth>
            <InputLabel id="topic-label">Help Topic</InputLabel>
            <Select
              labelId="topic-label"
              label="Help Topic"
              value={topicId}
              onChange={(e) => setTopicId(e.target.value as number | "")}
              data-testid="advanced-topic"
            >
              <MenuItem value="">
                <em>Any</em>
              </MenuItem>
              {topics.map((topic) => (
                <MenuItem key={topic.id} value={topic.id}>
                  {topic.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {/* Date Range */}
          <Stack direction="row" spacing={2}>
            <TextField
              label="Start Date"
              type="date"
              fullWidth
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              InputLabelProps={{ shrink: true }}
              inputProps={{ "data-testid": "advanced-start-date" }}
            />
            <TextField
              label="End Date"
              type="date"
              fullWidth
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              InputLabelProps={{ shrink: true }}
              inputProps={{ "data-testid": "advanced-end-date" }}
            />
          </Stack>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={handleReset} color="secondary">
          Reset
        </Button>
        <Button onClick={onClose}>Cancel</Button>
        <Button variant="contained" onClick={handleSubmit} data-testid="advanced-submit">
          Search
        </Button>
      </DialogActions>
    </Dialog>
  );
});
