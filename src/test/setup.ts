import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => {
  cleanup();
  window.history.replaceState({}, "", "/");
});

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query.includes("max-width: 1040px"),
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

Object.defineProperty(Element.prototype, "scrollIntoView", {
  writable: true,
  value: vi.fn(),
});

Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
  writable: true,
  value: vi.fn(() => null),
});
