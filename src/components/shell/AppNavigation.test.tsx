import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import { AppNavigation } from "./AppNavigation";

describe("AppNavigation", () => {
  it("shows only cloud setup destinations", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const { container } = render(
      <AppNavigation activeSection="live" onSelect={onSelect} />,
    );

    await user.click(screen.getByRole("tab", { name: "Translation" }));

    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith("translation");
    expect(screen.queryByRole("tab", { name: "Local LLM" })).not.toBeInTheDocument();
    expect((await axe(container)).violations).toEqual([]);
  });

  it("shows only local setup destinations", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <AppNavigation activeSection="live" experience="local" onSelect={onSelect} />,
    );

    await user.click(screen.getByRole("tab", { name: "Local LLM" }));

    expect(onSelect).toHaveBeenCalledWith("local_llm");
    expect(screen.queryByRole("tab", { name: "Translation" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Summary" })).not.toBeInTheDocument();
  });
});
