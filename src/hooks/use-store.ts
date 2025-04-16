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
  iterationScalingFactor: number;
};

type State = {
  params: FractalParams;
  colorScheme: string;
};

type Actions = {
  setFractalParams: (params: Partial<FractalParams>) => void;
  setColorScheme: (colorScheme: string) => void;
  resetFractalState: () => void;
};

export const initialFractalParamState: State = {
  params: {
    center: { x: -1, y: 0 },
    zoom: 1,
    maxIterations: 1000,
    iterationScalingFactor: 1000,
  },
  colorScheme: "default",
};

// Create the Zustand store
export const useFractalStore = create<State & Actions>()(
  persist(
    (set) => ({
      ...initialFractalParamState,

      setFractalParams: (newParams) =>
        set((state) => {
          const updatedParams = { ...state.params, ...newParams };
          return { params: updatedParams };
        }),
      setColorScheme: (colorScheme) => set({ colorScheme }),
      resetFractalState: () => set(initialFractalParamState),
    }),
    {
      name: "fractalwonder-store",
    }
  )
);

// helper function for derived "real" max iterations value
export const derivedRealIterations = (params: FractalParams): number => {
  const baseIterations = Math.max(1, params.maxIterations);
  const scaledIterations = params.iterationScalingFactor * Math.log10(params.zoom + 1);

  // const zoomFactorContribution = params.iterationScalingFactor * (params.zoom - initialFractalParamState.params.zoom);
  return Math.max(0, Math.round(baseIterations + scaledIterations));
};

export const getFractalParamState = useFractalStore.getState;
