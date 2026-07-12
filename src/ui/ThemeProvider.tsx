import { FluentProvider } from "@fluentui/react-components";
import { useEffect, useState, type PropsWithChildren } from "react";
import { darkTheme, lightTheme } from "./theme";

const darkModeQuery = "(prefers-color-scheme: dark)";

export function ApplicationThemeProvider({ children }: PropsWithChildren) {
  const [darkMode, setDarkMode] = useState(() => window.matchMedia(darkModeQuery).matches);

  useEffect(() => {
    const media = window.matchMedia(darkModeQuery);
    const updateTheme = (event: MediaQueryListEvent) => setDarkMode(event.matches);
    media.addEventListener("change", updateTheme);
    return () => media.removeEventListener("change", updateTheme);
  }, []);

  return (
    <FluentProvider className="application-theme" theme={darkMode ? darkTheme : lightTheme}>
      {children}
    </FluentProvider>
  );
}
