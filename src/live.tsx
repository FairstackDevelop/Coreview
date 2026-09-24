import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { api, type LiveStats, type PowerStats, type SmcSensors } from "./api";
import { useSettings } from "./settings";

export interface Point {
  t: number;
  cpu: number;
  mem: number;
  rx: number;
  tx: number;
  read: number;
  write: number;
  temp: number;
  freq: number;
  power: number | null;
  gpu: number | null;
}

export interface SensorStat {
  now: number;
  min: number;
  max: number;
  sum: number;
  n: number;
  hist: number[];
}

interface LiveCtx {
  latest: LiveStats | null;
  history: Point[];
  power: PowerStats | null;
  smc: SmcSensors | null;
  fanHist: Record<number, number[]>;
  sensors: Record<string, SensorStat>;
  resetSensors: () => void;
  paused: boolean;
  setPaused: (v: boolean) => void;
  toast: string | null;
}

const Ctx = createContext<LiveCtx>(null!);
export const useLive = () => useContext(Ctx);

function mergeStats(prev: Record<string, SensorStat>, entries: [string, number][]) {
  const next = { ...prev };
  for (const [label, value] of entries) {
    const o = prev[label];
    next[label] = {
      now: value,
      min: o ? Math.min(o.min, value) : value,
      max: o ? Math.max(o.max, value) : value,
      sum: (o?.sum ?? 0) + value,
      n: (o?.n ?? 0) + 1,
      hist: [...(o?.hist ?? []), value].slice(-90),
    };
  }
  return next;
}

export function LiveProvider({ children }: { children: ReactNode }) {
  const { settings, t } = useSettings();
  const [latest, setLatest] = useState<LiveStats | null>(null);
  const [history, setHistory] = useState<Point[]>([]);
  const [power, setPower] = useState<PowerStats | null>(null);
  const [smc, setSmc] = useState<SmcSensors | null>(null);
  const [fanHist, setFanHist] = useState<Record<number, number[]>>({});
  const [sensors, setSensors] = useState<Record<string, SensorStat>>({});
  const [paused, setPaused] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const lastAlert = useRef(0);
  const powerRef = useRef<PowerStats | null>(null);
  const cfg = useRef({ settings, t });
  cfg.current = { settings, t };

  const resetSensors = useCallback(() => setSensors({}), []);

  useEffect(() => {
    let alive = true;
    let timer = 0;
    const loop = async () => {
      let next = 2000;
      try {
        const p = await api.power();
        if (!alive) return;
        powerRef.current = p;
        setPower(p);
        next = p?.pollMs ?? 5000;
      } catch {}
      try {
        const s = await api.smc();
        if (!alive) return;
        setSmc(s);
        setSensors((prev) => mergeStats(prev, s.temps.map((x) => [`smc:${x.key}`, x.celsius])));
        setFanHist((prev) => {
          const next = { ...prev };
          for (const f of s.fans) next[f.id] = [...(next[f.id] ?? []), f.rpm].slice(-90);
          return next;
        });
      } catch {}
      if (alive) timer = window.setTimeout(loop, next);
    };
    loop();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, []);

  useEffect(() => {
    if (paused) return;
    let alive = true;
    const tick = async () => {
      try {
        const s = await api.live();
        if (!alive) return;
        const hottest = s.temps.reduce((m, x) => (x.celsius > m.celsius ? x : m), { label: "", celsius: 0 } as { label: string; celsius: number });
        setLatest(s);
        setSensors((prev) => mergeStats(prev, s.temps.map((x) => [x.label, x.celsius])));
        setHistory((h) =>
          [
            ...h,
            {
              t: s.timestamp,
              cpu: s.cpuTotal,
              mem: s.memTotal ? (s.memUsed / s.memTotal) * 100 : 0,
              rx: s.netRx,
              tx: s.netTx,
              read: s.diskRead,
              write: s.diskWrite,
              temp: hottest.celsius,
              freq: s.cpuMhz,
              power: powerRef.current?.watts ?? null,
              gpu: powerRef.current?.gpuLoad ?? null,
            },
          ].slice(-cfg.current.settings.history),
        );
        const { settings: st, t: tr } = cfg.current;
        if (st.alertOn && hottest.celsius >= st.alertTemp && Date.now() - lastAlert.current > 300_000) {
          lastAlert.current = Date.now();
          const body = tr("alert.temp", { label: hottest.label, temp: Math.round(hottest.celsius) });
          setToast(body);
          setTimeout(() => setToast(null), 6000);
          try {
            if ((await isPermissionGranted()) || (await requestPermission()) === "granted") {
              sendNotification({ title: "Fairstack Coreview", body });
            }
          } catch {}
        }
      } catch {}
    };
    tick();
    const id = setInterval(tick, settings.interval);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [paused, settings.interval]);

  return <Ctx.Provider value={{ latest, history, power, smc, fanHist, sensors, resetSensors, paused, setPaused, toast }}>{children}</Ctx.Provider>;
}
