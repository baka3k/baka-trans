import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "vitest-axe";
import App, { resolveApplicationRoute } from "./App";
import { ApplicationThemeProvider } from "./ui/ThemeProvider";

describe("application routes", () => {
  it.each([
    ["", "main"],
    ["?overlay=transparent", "transparent"],
    ["?overlay=look-help", "look-help"],
    ["?overlay=unknown", "main"],
  ] as const)("resolves %s to %s", (search, expected) => {
    expect(resolveApplicationRoute(search)).toBe(expected);
  });

  it.each([
    ["/", "heading", "Baka Trans"],
    ["/?overlay=transparent", "main", "Look Through"],
    ["/?overlay=look-help", "main", "Look & Help"],
  ])("renders %s without the native runtime", async (path, role, name) => {
    window.history.replaceState({}, "", path);
    const { container } = render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    expect(screen.getByRole(role, { name })).toBeInTheDocument();
    expect((await axe(container)).violations).toEqual([]);
  });

  it("preserves unsaved translation settings while changing destinations", async () => {
    const user = userEvent.setup();
    const { unmount } = render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    await user.click(screen.getByRole("tab", { name: "Translation" }));
    const keyInput = screen.getByPlaceholderText("Google Live Translation API key");
    await user.type(keyInput, "unsaved-key");
    await user.click(screen.getByRole("tab", { name: "Summary" }));
    await user.click(screen.getByRole("tab", { name: "Translation" }));

    expect(screen.getByDisplayValue("unsaved-key")).toBeInTheDocument();
    unmount();
  });

  it.each([
    ["/?overlay=transparent", "Look Through settings"],
    ["/?overlay=look-help", "Look & Help settings"],
  ])("exposes settings disclosure on %s", async (path, accessibleName) => {
    const user = userEvent.setup();
    window.history.replaceState({}, "", path);
    render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    await user.click(screen.getByRole("button", { name: "Show settings" }));

    expect(screen.getByRole("region", { name: accessibleName })).toBeInTheDocument();
  });
});
