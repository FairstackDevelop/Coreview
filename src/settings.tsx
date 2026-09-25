import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { api } from "./api";
import { bcpOf, makeTranslate, type Lang, type Translate } from "./i18n";

export type Theme = "dark" | "light" | "system";
export type Effect = "none" | "blur" | "mica";

export interface Settings {
  lang: Lang;
  theme: Theme;
  accent: string;
  effect: Effect;
  opacity: number;
  blur: number;
  radius: number;
  density: "compact" | "comfortable";
  font: number;
  motion: boolean;
  glow: boolean;
  interval: number;
  history: number;
  alertOn: boolean;
  alertTemp: number;
  stressLimit: number;
  historyOn: boolean;
  historyEvery: number;
  historyDays: number;
}

export const defaults: Settings = {
  lang: "en",
  theme: "dark",
  accent: "#7c8cff",
  effect: "blur",
  opacity: 55,
  blur: 24,
  radius: 18,
  density: "comfortable",
  font: 100,
  motion: true,
  glow: true,
  interval: 1000,
  history: 90,
  alertOn: true,
  alertTemp: 90,
  stressLimit: 95,
  historyOn: true,
  historyEvery: 5,
  historyDays: 7,
};

export const presets: Record<string, Partial<Settings>> = {
  glass: { theme: "dark", accent: "#7c8cff", effect: "blur", opacity: 55, blur: 24, radius: 18, glow: true },
  midnight: { theme: "dark", accent: "#22d3ee", effect: "none", opacity: 92, blur: 0, radius: 12, glow: false },
  paper: { theme: "light", accent: "#f97316", effect: "none", opacity: 96, blur: 0, radius: 10, glow: false },
};

export const IS_OVERLAY = new URLSearchParams(location.search).has("overlay");

const STORAGE_KEY = "coreview.settings";

function load(): Settings {
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") };
  } catch {
    return defaults;
  }
}

interface Ctx {
  settings: Settings;
  set: (patch: Partial<Settings>) => void;
  reset: () => void;
  t: Translate;
  locale: string;
}

const SettingsContext = createContext<Ctx>(null!);
export const useSettings = () => useContext(SettingsContext);
export const useT = () => useContext(SettingsContext).t;

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings>(load);
  const [systemDark, setSystemDark] = useState(() => matchMedia("(prefers-color-scheme: dark)").matches);

  useEffect(() => {
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const on = () => setSystemDark(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch {}
    const root = document.documentElement;
    const dark = settings.theme === "system" ? systemDark : settings.theme === "dark";
    root.dataset.theme = dark ? "dark" : "light";
    root.dataset.effect = settings.effect;
    root.dataset.density = settings.density;
    root.dataset.motion = settings.motion ? "on" : "off";
    root.dataset.glow = settings.glow ? "on" : "off";
    root.lang = settings.lang;
    root.style.setProperty("--accent", settings.accent);
    root.style.setProperty("--radius", `${settings.radius}px`);
    root.style.setProperty("--panel-alpha", `${settings.opacity}%`);
    root.style.setProperty("--blur", `${settings.blur}px`);
    root.style.fontSize = `${(16 * settings.font) / 100}px`;
  }, [settings, systemDark]);

  useEffect(() => {
    if (!IS_OVERLAY) api.windowEffect(settings.effect).catch(() => {});
  }, [settings.effect]);

  const set = useCallback((patch: Partial<Settings>) => setSettings((s) => ({ ...s, ...patch })), []);
  const reset = useCallback(() => setSettings((s) => ({ ...defaults, lang: s.lang })), []);
  const t = useMemo(() => makeTranslate(settings.lang), [settings.lang]);
  const value = useMemo(() => ({ settings, set, reset, t, locale: bcpOf(settings.lang) }), [settings, set, reset, t]);

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}
