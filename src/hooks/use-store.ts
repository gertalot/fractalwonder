import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Point = {
  x: number;
  y: number;
};

export type FractalParams = {
  center: Point;
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

export const initialFractalParamState: State = {
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
      params: initialFractalParamState.params,
      colorScheme: initialFractalParamState.colorScheme,
      setParams: (params: Partial<FractalParams>) =>
        set((state) => {
          return {
            params: { ...state.params, ...params },
          };
        }),
      setColorScheme: (colorScheme: string) => set({ colorScheme }),
      resetState: () => set(initialFractalParamState),
    }),
    { name: "fractalwonder-store" }
  )
);

export const getFractalParamState = useFractalStore.getState;
