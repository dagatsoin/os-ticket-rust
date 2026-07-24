// Shell health indicator. Reads HealthStore (which polls GET /api/health) and shows a
// coarse "Backend OK" / "Backend down" / "Checking…" chip (EPIC-M1-A AC-2).
import { useEffect } from "react";
import { observer } from "mobx-react-lite";
import { Chip, Tooltip } from "@mui/material";
import { useStores } from "../stores/StoreContext";

const LABEL = {
  ok: "Backend OK",
  down: "Backend down",
  unknown: "Checking…",
} as const;

const COLOR = {
  ok: "success",
  down: "error",
  unknown: "default",
} as const;

export const HealthIndicator = observer(function HealthIndicator() {
  const { health } = useStores();

  useEffect(() => {
    void health.check();
  }, [health]);

  return (
    <Tooltip title="Frontend → backend → database connectivity">
      <Chip
        size="small"
        color={COLOR[health.status]}
        label={LABEL[health.status]}
        data-testid="health-indicator"
        data-status={health.status}
      />
    </Tooltip>
  );
});
