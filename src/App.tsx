import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ask } from "@tauri-apps/plugin-dialog";
import { api, type HardwareInfo } from "./api";
import { LiveProvider, useLive } from "./live";
import Details from "./pages/Details";
import History from "./pages/History";
import MobilePage from "./pages/MobilePage";
import Monitor from "./pages/Monitor";
import OverlayPage from "./pages/OverlayPage";
import Network from "./pages/Network";
import Processes from "./pages/Processes";
import Sensors from "./pages/Sensors";
import Settings from "./pages/Settings";
import Snapshots from "./pages/Snapshots";
import Startup from "./pages/Startup";
import Storage from "./pages/Storage";
import Stress from "./pages/Stress";
import Summary from "./pages/Summary";
import { loadOverlay, saveOverlay } from "./overlayConfig";
import { SettingsProvider, useSettings } from "./settings";
import type { Key } from "./i18n";
import logo from "./assets/logo.svg";
import { Icon } from "./ui";

const pages = ["summary", "details", "monitor", "sensors", "history", "stress", "overlay", "mobile", "storage", "network", "processes", "startup", "snapshots", "settings"] as const;
type Page = (typeof pages)[number];

function Shell() {
  const { t, settings } = useSettings();
  const live = useLive();
  const { toast } = live;
  const liveRef = useRef(live);
  liveRef.current = live;
  const [page, setPage] = useState<Page>("summary");
  const [hw, setHw] = useState<HardwareInfo | null>(null);

  const load = useCallback(() => {
    api.hardware().then(setHw).catch(() => {});
  }, []);
  useEffect(load, [load]);

  useEffect(() => {
    (async () => {
      try {
        if (localStorage.getItem("coreview.driverAsked")) return;
        const s = await api.sensorStatus();
        if (s.platform !== "windows" || s.driver) return;
        localStorage.setItem("coreview.driverAsked", "1");
        const yes = await ask(`${t("sd.driverText")}\n\n${t("sd.askNow")}`, {
          title: t("sd.driverTitle"),
          kind: "info",
          okLabel: t("sd.install"),
          cancelLabel: t("common.cancel"),
        });
        if (yes) await api.installDriver();
      } catch {}
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!settings.historyOn) return;
    const id = setInterval(() => {
      const { latest, temps, power, smc, gpu } = liveRef.current;
      if (!latest) return;
      const maxOf = (pred: (g: string) => boolean) => {
        const v = temps.filter((x) => pred(x.group)).map((x) => x.celsius);
        return v.length ? Math.max(...v) : undefined;
      };
      const raw: Record<string, number | undefined> = {
        t: Date.now(),
        cpu: latest.cpuTotal,
        mem: latest.memTotal ? (latest.memUsed / latest.memTotal) * 100 : undefined,
        freq: latest.cpuMhz || undefined,
        tCpu: maxOf((g) => g === "chip"),
        tGpu: gpu?.temp ?? maxOf((g) => g === "gpu"),
        tMax: maxOf((g) => g !== "battery"),
        power: power?.watts ?? undefined,
        cpuW: power?.cpuWatts ?? undefined,
        gpuW: gpu?.power ?? power?.gpuWatts ?? undefined,
        gpu: gpu?.load ?? power?.gpuLoad ?? undefined,
        fan: smc && smc.fans.length ? Math.max(...smc.fans.map((f) => f.rpm)) : undefined,
        rx: latest.netRx,
        tx: latest.netTx,
      };
      const sample = Object.fromEntries(Object.entries(raw).filter(([, v]) => typeof v === "number" && Number.isFinite(v))) as Record<string, number>;
      api.historyAppend(sample).catch(() => {});
    }, settings.historyEvery * 1000);
    return () => clearInterval(id);
  }, [settings.historyOn, settings.historyEvery]);

  useEffect(() => {
    api.historyPrune(settings.historyDays, Date.now()).catch(() => {});
  }, [settings.historyDays]);

  useEffect(() => {
    const c = loadOverlay();
    if (c.enabled) api.overlayShow(c.corner, c.pos?.x ?? null, c.pos?.y ?? null, true).catch(() => {});
    const un = listen<{ visible: boolean; locked: boolean }>("overlay-state", (e) => {
      const cur = loadOverlay();
      if (cur.enabled !== e.payload.visible) saveOverlay({ ...cur, enabled: e.payload.visible });
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

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
              {page === "history" && <History />}
              {page === "overlay" && <OverlayPage />}
              {page === "mobile" && <MobilePage />}
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
