// Test helper: wrap a component in the providers it needs (MUI theme, MobX stores,
// react-router). Reused by B3/C4/D2 component tests.
import type { ReactElement, ReactNode } from "react";
import { render, type RenderOptions } from "@testing-library/react";
import { ThemeProvider } from "@mui/material/styles";
import { MemoryRouter } from "react-router-dom";
import { theme } from "../theme/theme";
import { RootStore } from "../stores/RootStore";
import { StoreProvider } from "../stores/StoreContext";

interface Options extends Omit<RenderOptions, "wrapper"> {
  store?: RootStore;
  route?: string;
}

export function renderWithProviders(
  ui: ReactElement,
  { store = new RootStore(), route = "/", ...options }: Options = {},
) {
  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <MemoryRouter
        initialEntries={[route]}
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <ThemeProvider theme={theme}>
          <StoreProvider store={store}>{children}</StoreProvider>
        </ThemeProvider>
      </MemoryRouter>
    );
  }
  return { store, ...render(ui, { wrapper: Wrapper, ...options }) };
}
