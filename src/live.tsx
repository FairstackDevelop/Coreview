import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { api, type LiveStats, type PowerStats } from "./api";
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
  sensors: Record<string, SensorStat>;
  resetSensors: () => void;
  paused: boolean;
  setPaused: (v: boolean) => void;
  toast: string | null;
}

const Ctx = createContext<LiveCtx>(null!);
export const useLive = () => useContext(Ctx);

export function LiveProvider({ children }: { children: ReactNode }) {
  const { settings, t } = useSettings();
  const [latest, setLatest] = useState<LiveStats | null>(null);
  const [history, setHistory] = useState<Point[]>([]);
  const [power, setPower] = useState<PowerStats | null>(null);
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
        setSensors((prev) => {
          const next = { ...prev };
          for (const x of s.temps) {
            const o = prev[x.label];
            next[x.label] = {
              now: x.celsius,
              min: o ? Math.min(o.min, x.celsius) : x.celsius,
              max: o ? Math.max(o.max, x.celsius) : x.celsius,
              sum: (o?.sum ?? 0) + x.celsius,
              n: (o?.n ?? 0) + 1,
              hist: [...(o?.hist ?? []), x.celsius].slice(-90),
            };
          }
          return next;
        });
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

  return <Ctx.Provider value={{ latest, history, power, sensors, resetSensors, paused, setPaused, toast }}>{children}</Ctx.Provider>;
}
