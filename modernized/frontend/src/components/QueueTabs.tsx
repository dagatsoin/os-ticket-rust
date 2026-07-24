// Queue tabs component (TS-M3-A4): renders the predefined queue tabs with count
// badges, handles tab clicks to update the URL, and highlights the active tab.
//
// @implements FS-020.2: queue tabs with counts
// @implements BS-020.4: tab visibility rules based on config and counts
import { observer } from "mobx-react-lite";
import { Badge, Tab, Tabs } from "@mui/material";
import type { QueueStats, QueueStatus } from "../stores/StaffTicketStore";

/** Tab configuration. */
interface TabConfig {
  value: QueueStatus;
  label: string;
  /** Returns the count to show in the badge. */
  getCount: (stats: QueueStats) => number;
  /** Returns true if the tab should be visible. */
  isVisible: (stats: QueueStats) => boolean;
}

/**
 * Tab definitions per FS-020.2:
 * - Open: always shown; count = open (+ answered if show_answered_tickets=1)
 * - Answered: shown only when show_answered_tickets=0 AND answered > 0
 * - My Tickets: shown only when assigned > 0
 * - Overdue: shown only when overdue > 0
 * - Closed: always shown
 */
const TAB_CONFIG: TabConfig[] = [
  {
    value: "open",
    label: "Open",
    getCount: (s) => (s.show_answered_tickets ? s.open + s.answered : s.open),
    isVisible: () => true,
  },
  {
    value: "answered",
    label: "Answered",
    getCount: (s) => s.answered,
    isVisible: (s) => !s.show_answered_tickets && s.answered > 0,
  },
  {
    value: "assigned",
    label: "My Tickets",
    getCount: (s) => s.assigned,
    isVisible: (s) => s.assigned > 0,
  },
  {
    value: "overdue",
    label: "Overdue",
    getCount: (s) => s.overdue,
    isVisible: (s) => s.overdue > 0,
  },
  {
    value: "closed",
    label: "Closed",
    getCount: (s) => s.closed,
    isVisible: () => true,
  },
];

export interface QueueTabsProps {
  /** Current active queue status (from URL). Defaults to "open" if absent. */
  activeStatus: QueueStatus | null;
  /** Quick stats from /api/staff/tickets/stats. */
  stats: QueueStats | null;
  /** Called when a tab is clicked; parent should update the URL. */
  onTabChange: (status: QueueStatus) => void;
}

/**
 * Queue tabs bar with count badges.
 * @implements FS-020.2: queue tabs with counts (TS-M3-A4)
 */
export const QueueTabs = observer(function QueueTabs({
  activeStatus,
  stats,
  onTabChange,
}: QueueTabsProps) {
  // Default to "open" if no status specified.
  const currentValue = activeStatus ?? "open";

  // If stats not loaded yet, show only Open and Closed tabs as placeholders.
  const visibleTabs = stats
    ? TAB_CONFIG.filter((tab) => tab.isVisible(stats))
    : TAB_CONFIG.filter((tab) => tab.value === "open" || tab.value === "closed");

  return (
    <Tabs
      value={currentValue}
      onChange={(_, value: QueueStatus) => onTabChange(value)}
      variant="scrollable"
      scrollButtons="auto"
      aria-label="Queue tabs"
      data-testid="queue-tabs"
    >
      {visibleTabs.map((tab) => {
        const count = stats ? tab.getCount(stats) : undefined;
        return (
          <Tab
            key={tab.value}
            value={tab.value}
            data-testid={`queue-tab-${tab.value}`}
            label={
              count !== undefined ? (
                <Badge
                  badgeContent={count}
                  color={tab.value === "overdue" ? "warning" : "primary"}
                  max={999}
                  showZero={tab.value === "open" || tab.value === "closed"}
                  data-testid={`queue-tab-badge-${tab.value}`}
                >
                  {tab.label}
                </Badge>
              ) : (
                tab.label
              )
            }
          />
        );
      })}
    </Tabs>
  );
});
