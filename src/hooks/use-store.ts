import { create } from "zustand";
import { persist } from "zustand/middleware";

type Params = {
  center: { x: number; y: number };
  zoom: number;
  maxIterations: number;
};

export type StoreState = {
  params: Params;
  colorScheme: string;
  setParams: (params: Partial<Params>) => void;
  setColorScheme: (colorScheme: string) => void;
};

export const useFractalStore = create<StoreState>()(
  persist(
    (set) => ({
      params: {
        center: { x: -1, y: 0 },
        zoom: 1,
        maxIterations: 250,
      },
      colorScheme: "default",
      setParams: (newParams) =>
        set((state) => ({
          params: { ...state.params, ...newParams },
        })),
      setColorScheme: (color) => set({ colorScheme: color }),
    }),
    {
      name: "fractalwonder-store",
    }
  )
);
