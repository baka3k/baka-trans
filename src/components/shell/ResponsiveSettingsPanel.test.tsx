import { createRef } from "react";
import { render, screen } from "@testing-library/react";
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
});
