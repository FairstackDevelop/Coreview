import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type FpsStats } from "../api";
import { mhz } from "../format";
import { useLive } from "../live";
import { loadOverlay, saveOverlay, useOverlayConfig } from "../overlayConfig";
import { useSettings } from "../settings";
import { Spark } from "../ui";

interface Row {
  key: string;
  label: string;
  value: string;
  level?: "warn" | "danger";
  extra?: number[];
}

const level = (v: number, warn: number, danger: number) => (v >= danger ? "danger" : v >= warn ? "warn" : undefined);

export default function OverlayView() {
  const { t, locale } = useSettings();
  const { latest, temps, power, smc } = useLive();
  const [cfg] = useOverlayConfig();
  const [locked, setLocked] = useState(true);
  const [fps, setFps] = useState<FpsStats | null>(null);
  const box = useRef<HTMLDivElement>(null);
  const lockedRef = useRef(true);
  lockedRef.current = locked;

  useEffect(() => {
    const un = listen<{ visible: boolean; locked: boolean }>("overlay-state", (e) => setLocked(e.payload.locked));
    const moved = getCurrentWindow().onMoved(({ payload }) => {
      if (lockedRef.current) return;
      const c = loadOverlay();
      saveOverlay({ ...c, pos: { x: payload.x, y: payload.y } });
      api.overlaySetCustom(payload.x, payload.y).catch(() => {});
    });
    return () => {
      un.then((f) => f());
      moved.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (!cfg.show.fps && !cfg.show.low && !cfg.show.frame) return;
    let alive = true;
    api.fpsEnable(true).catch(() => {});
    const id = setInterval(() => api.fpsStats().then((s) => alive && setFps(s)).catch(() => {}), 500);
    return () => {
      alive = false;
      clearInterval(id);
      api.fpsEnable(false).catch(() => {});
    };
  }, [cfg.show.fps, cfg.show.low, cfg.show.frame]);

  const rows = useMemo<Row[]>(() => {
    const out: Row[] = [];
    const s = cfg.show;
    const maxOf = (g: string) => Math.max(0, ...temps.filter((x) => x.group === g).map((x) => x.celsius));
    if (s.fps) out.push({ key: "fps", label: t("ov.fps"), value: fps ? String(Math.round(fps.fps)) : "—", level: fps && fps.fps < 30 ? "danger" : fps && fps.fps < 55 ? "warn" : undefined });
    if (s.low && fps) out.push({ key: "low", label: t("ov.low"), value: String(Math.round(fps.low1)) });
    if (s.frame && fps) out.push({ key: "frame", label: t("ov.frame"), value: `${fps.frameMs.toFixed(1)} ms`, extra: fps.history });
    if (s.cpuLoad && latest) out.push({ key: "cl", label: "CPU", value: `${Math.round(latest.cpuTotal)}%`, level: level(latest.cpuTotal, 80, 95) });
    if (s.cpuTemp && maxOf("chip") > 0) out.push({ key: "ct", label: t("ov.cpuTemp"), value: `${Math.round(maxOf("chip"))}°C`, level: level(maxOf("chip"), 80, 92) });
    if (s.cpuMhz && latest && latest.cpuMhz > 0) out.push({ key: "cm", label: t("mon.freq"), value: mhz(latest.cpuMhz, locale) });
    if (s.gpuLoad && power?.gpuLoad != null) out.push({ key: "gl", label: "GPU", value: `${Math.round(power.gpuLoad)}%`, level: level(power.gpuLoad, 90, 99) });
    if (s.gpuTemp && maxOf("gpu") > 0) out.push({ key: "gt", label: t("ov.gpuTemp"), value: `${Math.round(maxOf("gpu"))}°C`, level: level(maxOf("gpu"), 80, 90) });
    if (s.ram && latest && latest.memTotal) out.push({ key: "ram", label: "RAM", value: `${Math.round((latest.memUsed / latest.memTotal) * 100)}%`, level: level((latest.memUsed / latest.memTotal) * 100, 85, 95) });
    if (s.power) {
      const w = power?.watts ?? ((power?.cpuWatts ?? 0) + (power?.gpuWatts ?? 0) || null);
      if (w) out.push({ key: "pw", label: t("pow.system"), value: `${w.toFixed(0)} W` });
    }
    if (s.fan && smc && smc.fans.length) out.push({ key: "fan", label: t("fan.speed"), value: `${Math.round(Math.max(...smc.fans.map((f) => f.rpm)))} RPM` });
    return out;
  }, [cfg.show, latest, temps, power, smc, fps, t, locale]);

  useEffect(() => {
    if (!box.current) return;
    const el = box.current;
    const push = () => api.overlayResize(Math.ceil(el.offsetWidth) + 2, Math.ceil(el.offsetHeight) + 2).catch(() => {});
    const observer = new ResizeObserver(push);
    observer.observe(el);
    push();
    return () => observer.disconnect();
  }, [cfg.scale, cfg.layout]);

  return (
    <div className="ov-root">
      <div
        ref={box}
        className={`ov-box ${cfg.layout}`}
        data-locked={locked}
        data-tauri-drag-region
        style={{ fontSize: `${(13 * cfg.scale) / 100}px`, background: `rgba(9, 11, 17, ${cfg.opacity / 100})` }}
      >
        {rows.map((r) => (
          <div key={r.key} className={`ov-row ${r.level ?? ""}`} data-tauri-drag-region>
            <span data-tauri-drag-region>{r.label}</span>
            <b data-tauri-drag-region>{r.value}</b>
            {r.extra && r.extra.length > 1 && <Spark data={r.extra} color="var(--c3)" />}
          </div>
        ))}
      </div>
    </div>
  );
}
