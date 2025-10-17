import { Decimal } from "decimal.js";
import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Point = {
  x: Decimal;
  y: Decimal;
};

export type FractalParams = {
  center: Point;
  zoom: Decimal;
  maxIterations: number;
  iterationScalingFactor: number;
};

type State = {
  params: FractalParams;
  colorScheme: string;
  renderProgress: number;
  isUIVisible: boolean;
  algorithmMode: "standard" | "perturbation";
  lastRenderTime: number;
};

type Actions = {
  setFractalParams: (params: Partial<FractalParams>) => void;
  setColorScheme: (colorScheme: string) => void;
  setRenderProgress: (progress: number) => void;
  setUIVisible: (visible: boolean) => void;
  setAlgorithmMode: (mode: "standard" | "perturbation") => void;
  setLastRenderTime: (time: number) => void;
  resetFractalState: () => void;
};

export const initialFractalParamState: State = {
  params: {
    center: { x: new Decimal(-1), y: new Decimal(0) },
    zoom: new Decimal(1),
    maxIterations: 1000,
    iterationScalingFactor: 1000,
  },
  colorScheme: "default",
  renderProgress: 0,
  isUIVisible: true,
  algorithmMode: "perturbation",
  lastRenderTime: 0,
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
      setRenderProgress: (renderProgress) => set({ renderProgress }),
      setUIVisible: (isUIVisible) => set({ isUIVisible }),
      setAlgorithmMode: (algorithmMode) => set({ algorithmMode }),
      setLastRenderTime: (lastRenderTime) => set({ lastRenderTime }),
      resetFractalState: () => set(initialFractalParamState),
    }),
    {
      name: "fractalwonder-store",
      partialize: (state) => ({
        ...state,
        params: {
          ...state.params,
          center: {
            x: state.params.center.x.toString(),
            y: state.params.center.y.toString(),
          },
          zoom: state.params.zoom instanceof Decimal ? state.params.zoom.toString() : state.params.zoom,
        },
      }),
      onRehydrateStorage: () => (state) => {
        if (state?.params?.center) {
          state.params.center = {
            x: new Decimal(state.params.center.x),
            y: new Decimal(state.params.center.y),
          };
        }
        if (state?.params?.zoom && typeof state.params.zoom === 'string') {
          state.params.zoom = new Decimal(state.params.zoom);
        } else if (state?.params?.zoom && typeof state.params.zoom === 'number') {
          state.params.zoom = new Decimal(state.params.zoom);
        }
      },
    }
  )
);

// helper function for derived "real" max iterations value
export const derivedRealIterations = (params: FractalParams): number => {
  const baseIterations = Math.max(1, params.maxIterations);
  const zoomValue = params.zoom instanceof Decimal ? params.zoom.toNumber() : params.zoom;
  const logValue = Math.log10(zoomValue + 1);
  
  // Piecewise function: gentle slope until ~250k zoom, then accelerate
  // At zoom 250k: log10(250001) ≈ 5.4
  const threshold = Math.log10(250000);
  
  let scaledIterations;
  if (logValue <= threshold) {
    // Below 250k zoom: use gentler exponent (linear-ish growth)
    // At zoom 1: 0.3^1.5 = 0.16 → 160 iterations
    // At zoom 100: 2^1.5 = 2.8 → 2.8k iterations  
    // At zoom 250k: 5.4^1.5 = 12.5 → 12.5k iterations
    scaledIterations = params.iterationScalingFactor * Math.pow(logValue, 1.5);
  } else {
    // Above 250k zoom: add aggressive scaling on top of base
    const baseAt250k = params.iterationScalingFactor * Math.pow(threshold, 1.5);
    const excessLog = logValue - threshold;
    // At zoom 1e9: log=9, excess=3.6, 3.6^3 = 46.6 → base(12.5k) + 46.6k = ~59k iterations
    const additionalIterations = params.iterationScalingFactor * Math.pow(excessLog, 3);
    scaledIterations = baseAt250k + additionalIterations;
  }

  return Math.max(0, Math.round(baseIterations + scaledIterations));
};

export const getFractalParamState = useFractalStore.getState;
