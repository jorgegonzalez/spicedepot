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
