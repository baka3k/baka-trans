import { createRef } from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { ResponsiveSettingsPanel } from "./ResponsiveSettingsPanel";

describe("ResponsiveSettingsPanel", () => {
  it("closes on Escape and restores focus to its trigger", async () => {
    const user = userEvent.setup();
    const triggerRef = createRef<HTMLButtonElement>();
    const onClose = vi.fn();
    const { container } = render(
      <>
        <button ref={triggerRef}>Settings trigger</button>
        <ResponsiveSettingsPanel
          open
          section="audio"
          onClose={onClose}
          returnFocusRef={triggerRef}
        >
          <button>First setting</button>
        </ResponsiveSettingsPanel>
      </>,
    );

    expect(screen.getByRole("dialog", { name: "Audio settings" })).toBeInTheDocument();
    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(triggerRef.current).toHaveFocus();
    expect((await axe(container)).violations).toEqual([]);
  });

  it("labels the Local LLM destination explicitly", () => {
    const triggerRef = createRef<HTMLButtonElement>();
    render(
      <ResponsiveSettingsPanel
        open
        section="local_llm"
        onClose={() => undefined}
        returnFocusRef={triggerRef}
      >
        <button>First local setting</button>
      </ResponsiveSettingsPanel>,
    );

    expect(screen.getByRole("dialog", { name: "Local LLM settings" })).toBeInTheDocument();
  });

  it("traps Local LLM keyboard focus without including hidden settings", async () => {
    const user = userEvent.setup();
    const triggerRef = createRef<HTMLButtonElement>();
    render(
      <ResponsiveSettingsPanel
        open
        section="local_llm"
        onClose={() => undefined}
        returnFocusRef={triggerRef}
      >
        <div style={{ display: "none" }}>
          <button>Hidden audio setting</button>
        </div>
        <button>Last visible local setting</button>
      </ResponsiveSettingsPanel>,
    );

    const dialog = screen.getByRole("dialog", { name: "Local LLM settings" });
    const close = within(dialog).getByRole("button", { name: "Close settings" });
    const lastVisible = screen.getByRole("button", { name: "Last visible local setting" });
    expect(close).toHaveFocus();

    await user.tab({ shift: true });
    expect(lastVisible).toHaveFocus();
    await user.tab();
    expect(close).toHaveFocus();
  });
});
