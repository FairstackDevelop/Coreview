import { useEffect, useState } from "react";

export interface OverlayConfig {
  enabled: boolean;
  corner: "tl" | "tr" | "bl" | "br";
  layout: "vertical" | "horizontal";
  scale: number;
  opacity: number;
  pos: { x: number; y: number } | null;
  show: {
    fps: boolean;
    low: boolean;
    frame: boolean;
    cpuLoad: boolean;
    cpuTemp: boolean;
    cpuMhz: boolean;
    gpuLoad: boolean;
    gpuTemp: boolean;
    ram: boolean;
    power: boolean;
    fan: boolean;
  };
}

export const overlayDefaults: OverlayConfig = {
  enabled: false,
  corner: "tl",
  layout: "vertical",
  scale: 100,
  opacity: 55,
  pos: null,
  show: { fps: true, low: true, frame: true, cpuLoad: true, cpuTemp: true, cpuMhz: true, gpuLoad: true, gpuTemp: true, ram: true, power: false, fan: false },
};

const KEY = "coreview.overlay";

export function loadOverlay(): OverlayConfig {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    return { ...overlayDefaults, ...saved, show: { ...overlayDefaults.show, ...(saved.show ?? {}) } };
  } catch {
    return overlayDefaults;
  }
}

export function saveOverlay(cfg: OverlayConfig) {
  try {
    localStorage.setItem(KEY, JSON.stringify(cfg));
  } catch {}
  window.dispatchEvent(new CustomEvent("coreview-overlay"));
}

export function useOverlayConfig(): [OverlayConfig, (patch: Partial<OverlayConfig>) => void] {
  const [cfg, setCfg] = useState<OverlayConfig>(loadOverlay);
  useEffect(() => {
    const refresh = () => setCfg(loadOverlay());
    window.addEventListener("storage", refresh);
    window.addEventListener("coreview-overlay", refresh);
    const id = setInterval(refresh, 1500);
    return () => {
      window.removeEventListener("storage", refresh);
      window.removeEventListener("coreview-overlay", refresh);
      clearInterval(id);
    };
  }, []);
  const update = (patch: Partial<OverlayConfig>) => saveOverlay({ ...loadOverlay(), ...patch });
  return [cfg, update];
}
