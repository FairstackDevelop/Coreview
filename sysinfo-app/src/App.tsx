import { useCallback, useEffect, useState } from "react";
import { api, type HardwareInfo } from "./api";
import { LiveProvider, useLive } from "./live";
import Details from "./pages/Details";
import Monitor from "./pages/Monitor";
import Network from "./pages/Network";
import Processes from "./pages/Processes";
import Sensors from "./pages/Sensors";
import Settings from "./pages/Settings";
import Snapshots from "./pages/Snapshots";
import Startup from "./pages/Startup";
import Storage from "./pages/Storage";
import Stress from "./pages/Stress";
import Summary from "./pages/Summary";
import { SettingsProvider, useSettings } from "./settings";
import type { Key } from "./i18n";
import logo from "./assets/logo.svg";
import { Icon } from "./ui";

const pages = ["summary", "details", "monitor", "sensors", "stress", "storage", "network", "processes", "startup", "snapshots", "settings"] as const;
type Page = (typeof pages)[number];

function Shell() {
  const { t } = useSettings();
  const { toast } = useLive();
  const [page, setPage] = useState<Page>("summary");
  const [hw, setHw] = useState<HardwareInfo | null>(null);

  const load = useCallback(() => {
    api.hardware().then(setHw).catch(() => {});
  }, []);
  useEffect(load, [load]);

  const needsHw = page === "summary" || page === "storage" || page === "network" || page === "snapshots";

  return (
    <div className="app" data-platform={navigator.userAgent.includes("Mac") ? "mac" : "other"}>
      <aside className="sidebar" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          <img className="logo" src={logo} alt="" />
          Fairstack Coreview
        </div>
        <nav>
          {pages.map((p) => (
            <button key={p} className={page === p ? "active" : ""} onClick={() => setPage(p)}>
              <Icon name={p} />
              {t(`nav.${p}` as Key)}
            </button>
          ))}
        </nav>
      </aside>
      <main>
        <div className="drag" data-tauri-drag-region />
        <div className="content" key={page}>
          {needsHw && !hw ? (
            <div className="empty">{t("common.loading")}</div>
          ) : (
            <>
              {page === "summary" && hw && <Summary hw={hw} reload={load} />}
              {page === "details" && <Details />}
              {page === "monitor" && <Monitor />}
              {page === "sensors" && <Sensors />}
              {page === "stress" && <Stress />}
              {page === "storage" && hw && <Storage hw={hw} />}
              {page === "network" && hw && <Network hw={hw} />}
              {page === "processes" && <Processes />}
              {page === "startup" && <Startup />}
              {page === "snapshots" && hw && <Snapshots hw={hw} />}
              {page === "settings" && <Settings />}
            </>
          )}
        </div>
      </main>
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

export default function App() {
  return (
    <SettingsProvider>
      <LiveProvider>
        <Shell />
      </LiveProvider>
    </SettingsProvider>
  );
}
