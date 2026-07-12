import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { SessionCommandBar } from "./SessionCommandBar";

const options = [
  { value: "auto", label: "Auto" },
  { value: "en", label: "English" },
  { value: "vi", label: "Vietnamese" },
];

function renderCommandBar(overrides: Partial<React.ComponentProps<typeof SessionCommandBar>> = {}) {
  const props: React.ComponentProps<typeof SessionCommandBar> = {
    sourceLanguage: "auto",
    targetLanguage: "vi",
    sourceOptions: options,
    targetOptions: options,
    onSourceChange: vi.fn(),
    onTargetChange: vi.fn(),
    fallbackEnabled: false,
    onFallbackChange: vi.fn(),
    canStart: true,
    canPause: false,
    canResume: false,
    canStop: false,
    canTranslateNow: false,
    busy: false,
    paused: false,
    readinessLabel: "Ready",
    onStart: vi.fn(),
    onPause: vi.fn(),
    onResume: vi.fn(),
    onStop: vi.fn(),
    onTranslateNow: vi.fn(),
    ...overrides,
  };
  return { props, ...render(<SessionCommandBar {...props} />) };
}

describe("SessionCommandBar", () => {
  it("invokes Start exactly once", async () => {
    const user = userEvent.setup();
    const result = renderCommandBar();

    await user.click(screen.getByRole("button", { name: "Start" }));

    expect(result.props.onStart).toHaveBeenCalledTimes(1);
    expect((await axe(result.container)).violations).toEqual([]);
  });

  it("uses one stable Pause or Resume action slot", () => {
    renderCommandBar({ paused: true, canResume: true });

    expect(screen.getByRole("button", { name: "Resume" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "Pause" })).not.toBeInTheDocument();
  });

  it("invokes each active meeting command exactly once", async () => {
    const user = userEvent.setup();
    const result = renderCommandBar({
      canPause: true,
      canStop: true,
      canTranslateNow: true,
    });

    await user.click(screen.getByRole("button", { name: "Pause" }));
    await user.click(screen.getByRole("button", { name: "Stop" }));
    await user.click(screen.getByRole("button", { name: "Translate now" }));

    expect(result.props.onPause).toHaveBeenCalledTimes(1);
    expect(result.props.onStop).toHaveBeenCalledTimes(1);
    expect(result.props.onTranslateNow).toHaveBeenCalledTimes(1);
  });

  it("invokes Resume exactly once while paused", async () => {
    const user = userEvent.setup();
    const result = renderCommandBar({ paused: true, canResume: true });

    await user.click(screen.getByRole("button", { name: "Resume" }));

    expect(result.props.onResume).toHaveBeenCalledTimes(1);
  });
});
