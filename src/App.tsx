import { useState } from "react";
import MainApp from "./app/MainApp";
import { LookHelpOverlayWindow } from "./app/LookHelpOverlayWindow";
import { ModeChooser, type ApplicationMode } from "./app/ModeChooser";
import { TransparentOverlayWindow } from "./app/TransparentOverlayWindow";

export type ApplicationRoute = "main" | "transparent" | "look-help";

export function resolveApplicationRoute(search: string): ApplicationRoute {
  const overlay = new URLSearchParams(search).get("overlay");
  if (overlay === "transparent") {
    return "transparent";
  }
  if (overlay === "look-help") {
    return "look-help";
  }
  return "main";
}

export default function App() {
  const route = resolveApplicationRoute(window.location.search);
  const [mode, setMode] = useState<ApplicationMode | null>(null);

  if (route === "transparent") {
    return <TransparentOverlayWindow />;
  }
  if (route === "look-help") {
    return <LookHelpOverlayWindow />;
  }
  if (mode === null) {
    return <ModeChooser onSelect={setMode} />;
  }
  return (
    <MainApp
      experience={mode}
      key={mode}
      onRequestModeChange={() => setMode(null)}
    />
  );
}
