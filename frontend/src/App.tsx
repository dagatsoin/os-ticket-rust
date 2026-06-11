// App root: wires the RootStore (with a router-aware 401 redirect), MUI theme, and the
// single router. The apiClient's onUnauthorized navigates via react-router so a 401 sends
// the user to the matching realm login without a full page reload.
import { useMemo } from "react";
import { ThemeProvider, CssBaseline } from "@mui/material";
import { BrowserRouter, useNavigate } from "react-router-dom";
import { theme } from "./theme/theme";
import { RootStore } from "./stores/RootStore";
import { StoreProvider } from "./stores/StoreContext";
import { AppRoutes } from "./routes/router";

function AppInner() {
  const navigate = useNavigate();
  // One store per app session; 401s redirect via router instead of window.location.
  const store = useMemo(
    () => new RootStore({ onUnauthorized: (loginPath) => navigate(loginPath) }),
    [navigate],
  );
  return (
    <StoreProvider store={store}>
      <AppRoutes />
    </StoreProvider>
  );
}

export function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AppInner />
      </BrowserRouter>
    </ThemeProvider>
  );
}
