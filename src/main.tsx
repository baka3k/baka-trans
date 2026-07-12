import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ApplicationThemeProvider } from "./ui/ThemeProvider";
import "./styles/app.css";
import "./styles/overlays.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ApplicationThemeProvider>
      <App />
    </ApplicationThemeProvider>
  </React.StrictMode>,
);

