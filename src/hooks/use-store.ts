import { create } from "zustand";
import { persist } from "zustand/middleware";

export type FractalParams = {
  center: { x: number; y: number };
  zoom: number;
  maxIterations: number;
};

type State = {
  params: FractalParams;
  colorScheme: string;
};

type Actions = {
  setParams: (params: Partial<FractalParams>) => void;
  setColorScheme: (colorScheme: string) => void;
  resetState: () => void;
};

const initialState: State = {
  params: {
    center: { x: -1, y: 0 },
    zoom: 1,
    maxIterations: 250,
  },
  colorScheme: "default",
};

export const useFractalStore = create<State & Actions>()(
  persist(
    (set) => ({
      params: initialState.params,
      colorScheme: initialState.colorScheme,
      setParams: (params: Partial<FractalParams>) =>
        set((state) => {
          return {
            params: { ...state.params, ...params },
          };
        }),
      setColorScheme: (colorScheme: string) => set({ colorScheme }),
      resetState: () => set(initialState),
    }),
    { name: "fractalwonder-store" }
  )
);

export const getFractalParamState = useFractalStore.getState;
