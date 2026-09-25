import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { LiveProvider } from "./live";
import OverlayView from "./overlay/OverlayView";
import { IS_OVERLAY, SettingsProvider } from "./settings";
import "./styles.css";

if (IS_OVERLAY) document.documentElement.dataset.overlay = "1";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {IS_OVERLAY ? (
      <SettingsProvider>
        <LiveProvider alerts={false}>
          <OverlayView />
        </LiveProvider>
      </SettingsProvider>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
