// React context wiring for the RootStore. Components read stores via useStores();
// tests wrap with StoreProvider and an injected RootStore.
import { createContext, useContext, type ReactNode } from "react";
import { RootStore } from "./RootStore";

const StoreContext = createContext<RootStore | null>(null);

export function StoreProvider({
  store,
  children,
}: {
  store: RootStore;
  children: ReactNode;
}) {
  return <StoreContext.Provider value={store}>{children}</StoreContext.Provider>;
}

// eslint-disable-next-line react-refresh/only-export-components -- provider + hook colocated by design
export function useStores(): RootStore {
  const store = useContext(StoreContext);
  if (!store) {
    throw new Error("useStores must be used within a <StoreProvider>");
  }
  return store;
}
