import { create } from "zustand";
import { persist } from "zustand/middleware";

interface ActiveConnectionState {
  activeConnectionId: string | null;
  setActiveConnectionId: (id: string | null) => void;
}

export const useActiveConnection = create<ActiveConnectionState>()(
  persist(
    (set) => ({
      activeConnectionId: null,
      setActiveConnectionId: (id) => set({ activeConnectionId: id }),
    }),
    { name: "spicelens.active-connection" },
  ),
);

export type Theme = "dark" | "light";

interface SchemaDraftState {
  drafts: Record<string, string>;
  saveDraft: (id: string, text: string) => void;
  removeDraft: (id: string) => void;
}

export const useSchemaDrafts = create<SchemaDraftState>()(persist((set) => ({
  drafts: {},
  saveDraft: (id, text) => set((state) => ({ drafts: { ...state.drafts, [id]: text } })),
  removeDraft: (id) => set((state) => {
    const drafts = { ...state.drafts };
    delete drafts[id];
    return { drafts };
  }),
}), { name: "spicedepot.schema-drafts" }));

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

export const useTheme = create<ThemeState>()(
  persist(
    (set, get) => ({
      theme: "dark",
      setTheme: (theme) => set({ theme }),
      toggleTheme: () =>
        set({ theme: get().theme === "dark" ? "light" : "dark" }),
    }),
    { name: "spicelens.theme" },
  ),
);
