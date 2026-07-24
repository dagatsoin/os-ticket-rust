// Global success/error snackbar (TS-M4-A0). Reads the shared SnackbarStore and
// renders a single MUI Snackbar + Alert; every admin CRUD screen and route guard
// pushes notices through the store rather than owning its own toast.
import { observer } from "mobx-react-lite";
import { Alert, Snackbar } from "@mui/material";
import { useStores } from "../stores/StoreContext";

export const AppSnackbar = observer(function AppSnackbar() {
  const { snackbar } = useStores();
  return (
    <Snackbar
      open={snackbar.open}
      autoHideDuration={5000}
      onClose={(_e, reason) => {
        if (reason !== "clickaway") snackbar.close();
      }}
      anchorOrigin={{ vertical: "top", horizontal: "center" }}
    >
      <Alert
        severity={snackbar.severity}
        variant="filled"
        onClose={snackbar.close}
        sx={{ width: "100%" }}
        data-testid="app-snackbar"
      >
        {snackbar.message}
      </Alert>
    </Snackbar>
  );
});
