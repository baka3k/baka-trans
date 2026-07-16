import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { AppNavigation } from "./AppNavigation";

describe("AppNavigation", () => {
  it("changes setup destinations without invoking unrelated work", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const { container } = render(
      <AppNavigation activeSection="live" onSelect={onSelect} />,
    );

    await user.click(screen.getByRole("tab", { name: "Translation" }));

    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith("translation");
    await user.click(screen.getByRole("tab", { name: "Local LLM" }));
    expect(onSelect).toHaveBeenLastCalledWith("local_llm");
    expect((await axe(container)).violations).toEqual([]);
  });
});
