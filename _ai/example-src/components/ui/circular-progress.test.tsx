import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CircularProgress } from "./circular-progress";

describe("CircularProgress", () => {
  it("should render an SVG with correct dimensions", () => {
    const { container } = render(<CircularProgress value={50} />);
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("viewBox", "0 0 24 24");
    expect(svg).toHaveClass("size-6");
  });

  it("should have correct ARIA attributes", () => {
    const { container } = render(<CircularProgress value={75} />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("role", "progressbar");
    expect(svg).toHaveAttribute("aria-valuenow", "75");
    expect(svg).toHaveAttribute("aria-valuemin", "0");
    expect(svg).toHaveAttribute("aria-valuemax", "100");
    expect(svg).toHaveAttribute("aria-label", "Progress: 75%");
  });

  it("should render empty circle at 0% progress", () => {
    const { container } = render(<CircularProgress value={0} />);
    const circle = container.querySelector("circle");
    expect(circle).toBeInTheDocument();
    
    // Should have background circle but no fill path or filled circle
    const path = container.querySelector("path");
    expect(path).toBeNull();
    
    const circles = container.querySelectorAll("circle");
    expect(circles.length).toBe(1); // Only background circle
  });

  it("should render half-filled circle at 50% progress", () => {
    const { container } = render(<CircularProgress value={50} />);
    const path = container.querySelector("path");
    expect(path).toBeInTheDocument();
    expect(path).toHaveAttribute("fill", "currentColor");
  });

  it("should render fully-filled circle at 100% progress", () => {
    const { container } = render(<CircularProgress value={100} />);
    
    // At 100%, should use a filled circle instead of a path
    const circles = container.querySelectorAll("circle");
    expect(circles.length).toBe(2); // Background circle + filled circle
    
    const filledCircle = circles[1];
    expect(filledCircle).toHaveAttribute("fill", "currentColor");
    
    const path = container.querySelector("path");
    expect(path).toBeNull();
  });

  it("should render correct pie chart at 25% progress", () => {
    const { container } = render(<CircularProgress value={25} />);
    const path = container.querySelector("path");
    expect(path).toBeInTheDocument();
    
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "25");
  });

  it("should render correct pie chart at 75% progress", () => {
    const { container } = render(<CircularProgress value={75} />);
    const path = container.querySelector("path");
    expect(path).toBeInTheDocument();
    
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "75");
  });

  it("should respond to value prop changes", () => {
    const { container, rerender } = render(<CircularProgress value={25} />);
    let svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "25");
    
    rerender(<CircularProgress value={75} />);
    svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "75");
  });

  it("should accept and apply className prop", () => {
    const { container } = render(<CircularProgress value={50} className="custom-class" />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveClass("size-6");
    expect(svg).toHaveClass("custom-class");
  });

  it("should clamp values above 100", () => {
    const { container } = render(<CircularProgress value={150} />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "100");
  });

  it("should clamp values below 0", () => {
    const { container } = render(<CircularProgress value={-10} />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-valuenow", "0");
  });
});

