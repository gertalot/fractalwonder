import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CircularProgressIndicator } from "./CircularProgressIndicator";

// Mock the Zustand store
vi.mock("@/hooks/use-store", () => ({
  useFractalStore: vi.fn(),
}));

const { useFractalStore } = await import("@/hooks/use-store");

describe("CircularProgressIndicator", () => {
  it("should not render when renderProgress is 0", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 0, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });

  it("should not render when renderProgress is 100", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 100, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });

  it("should render when renderProgress is between 1 and 99", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 50, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("aria-valuenow", "50");
  });

  it("should render when renderProgress is 1", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 1, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("aria-valuenow", "1");
  });

  it("should render when renderProgress is 99", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 99, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("aria-valuenow", "99");
  });

  it("should have correct CSS classes for positioning", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 50, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const wrapper = container.querySelector("div");
    expect(wrapper).toHaveClass("fixed");
    expect(wrapper).toHaveClass("bottom-4");
    expect(wrapper).toHaveClass("left-4");
    expect(wrapper).toHaveClass("z-40");
  });

  it("should pass correct progress value to CircularProgress component", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 75, isUIVisible: false })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "75");
  });

  it("should not render when isUIVisible is true", () => {
    vi.mocked(useFractalStore).mockImplementation((selector: (state: unknown) => unknown) => 
      selector({ renderProgress: 50, isUIVisible: true })
    );
    
    const { container } = render(<CircularProgressIndicator />);
    const svg = container.querySelector("svg");
    expect(svg).toBeNull();
  });
});

