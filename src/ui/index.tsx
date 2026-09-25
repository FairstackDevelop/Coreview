import type { CSSProperties, ReactNode } from "react";
import { Area, AreaChart, CartesianGrid, Line, LineChart, ReferenceLine, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";

export function Card({ title, icon, right, children, className = "" }: { title?: string; icon?: ReactNode; right?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={`card ${className}`}>
      {(title || right) && (
        <header className="card-head">
          <h3>
            {icon}
            {title}
          </h3>
          {right}
        </header>
      )}
      {children}
    </section>
  );
}

export function Row({ label, value }: { label: string; value?: ReactNode }) {
  if (value === undefined || value === null || value === "" || value === "0") return null;
  return (
    <div className="row">
      <span>{label}</span>
      <b>{value}</b>
    </div>
  );
}

export function Bar({ value, tone }: { value: number; tone?: "warn" | "danger" }) {
  const v = Math.max(0, Math.min(100, value));
  const auto = tone ?? (v > 90 ? "danger" : v > 75 ? "warn" : undefined);
  return (
    <div className="bar">
      <i className={auto} style={{ width: `${v}%` }} />
    </div>
  );
}

export function Ring({ value, label, sub, size = 132 }: { value: number; label: string; sub?: string; size?: number }) {
  const v = Math.max(0, Math.min(100, value));
  const r = size / 2 - 10;
  const c = 2 * Math.PI * r;
  const cls = v > 90 ? "danger" : v > 75 ? "warn" : "";
  return (
    <div className="ring" style={{ width: size, height: size }}>
      <svg width={size} height={size}>
        <circle cx={size / 2} cy={size / 2} r={r} className="ring-track" />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          className={`ring-fill ${cls}`}
          strokeDasharray={c}
          strokeDashoffset={c * (1 - v / 100)}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
      <div className="ring-text">
        <strong>{label}</strong>
        {sub && <small>{sub}</small>}
      </div>
    </div>
  );
}

interface SeriesDef {
  key: string;
  color: string;
  name: string;
}

export function Chart({
  data,
  series,
  format,
  max,
  height = 150,
}: {
  data: object[];
  series: SeriesDef[];
  format: (v: number) => string;
  max?: number;
  height?: number;
}) {
  return (
    <div style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 6, right: 0, bottom: 0, left: 0 }}>
          <defs>
            {series.map((s) => (
              <linearGradient key={s.key} id={`g-${s.key}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={s.color} stopOpacity={0.5} />
                <stop offset="100%" stopColor={s.color} stopOpacity={0} />
              </linearGradient>
            ))}
          </defs>
          <YAxis hide domain={[0, max ?? "auto"]} />
          <Tooltip
            cursor={{ stroke: "var(--muted)", strokeDasharray: "3 3" }}
            contentStyle={{ background: "var(--tooltip)", border: "1px solid var(--line)", borderRadius: 12, fontSize: 12 }}
            labelFormatter={() => ""}
            formatter={(v, n) => [format(Number(v)), n]}
          />
          {series.map((s) => (
            <Area
              key={s.key}
              type="monotone"
              dataKey={s.key}
              name={s.name}
              stroke={s.color}
              strokeWidth={2}
              fill={`url(#g-${s.key})`}
              isAnimationActive={false}
            />
          ))}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

export function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button role="switch" aria-checked={checked} className={`toggle ${checked ? "on" : ""}`} onClick={() => onChange(!checked)}>
      <i />
    </button>
  );
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="seg">
      {options.map((o) => (
        <button key={o.value} className={o.value === value ? "active" : ""} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Slider({ value, min, max, step = 1, suffix = "", onChange }: { value: number; min: number; max: number; step?: number; suffix?: string; onChange: (v: number) => void }) {
  const style = { "--fill": `${((value - min) / (max - min)) * 100}%` } as CSSProperties;
  return (
    <div className="slider">
      <input type="range" min={min} max={max} step={step} value={value} style={style} onChange={(e) => onChange(Number(e.target.value))} />
      <output>
        {value}
        {suffix}
      </output>
    </div>
  );
}

export function Badge({ tone, children }: { tone: "ok" | "warn" | "danger" | "muted"; children: ReactNode }) {
  return <span className={`badge ${tone}`}>{children}</span>;
}

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="field">
      <label>{label}</label>
      {children}
    </div>
  );
}

export function PageHead({ title, sub, right }: { title: string; sub?: string; right?: ReactNode }) {
  return (
    <div className="page-head">
      <div>
        <h1>{title}</h1>
        {sub && <p>{sub}</p>}
      </div>
      <div className="page-actions">{right}</div>
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return <div className="empty">{children}</div>;
}

export function Icon({ name }: { name: string }) {
  const paths: Record<string, string> = {
    summary: "M4 5h7v7H4zM13 5h7v4h-7zM13 11h7v8h-7zM4 14h7v5H4z",
    sensors: "M14 14.8V4a2 2 0 10-4 0v10.8a4 4 0 104 0z",
    stress: "M13 2L4 14h7l-1 8 9-12h-7z",
    info: "M12 22a10 10 0 100-20 10 10 0 000 20zM12 16v-4M12 8h.01",
    mobile: "M8 2h8a2 2 0 012 2v16a2 2 0 01-2 2H8a2 2 0 01-2-2V4a2 2 0 012-2zM11 18h2",
    history: "M3 12a9 9 0 109-9 9.75 9.75 0 00-6.74 2.74L3 8M3 3v5h5M12 7v5l4 2",
    overlay: "M3 5h18v12H3zM8 21h8M12 17v4M7 10h4M7 13h7",
    details: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01",
    plus: "M12 5v14M5 12h14",
    monitor: "M3 12h4l3-8 4 16 3-8h4",
    storage: "M4 7c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3zM4 7v10c0 1.7 3.6 3 8 3s8-1.3 8-3V7M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
    network: "M12 3a9 9 0 100 18 9 9 0 000-18zM3 12h18M12 3c2.5 2.5 3.8 5.5 3.8 9S14.5 18.5 12 21c-2.5-2.5-3.8-5.5-3.8-9S9.5 5.5 12 3z",
    processes: "M5 4h14v6H5zM5 14h14v6H5zM8 7h.01M8 17h.01",
    startup: "M12 3v9M6.5 6.5a8 8 0 1011 0",
    snapshots: "M4 7h3l2-3h6l2 3h3v12H4zM12 17a4 4 0 100-8 4 4 0 000 8z",
    settings: "M12 15a3 3 0 100-6 3 3 0 000 6zM19.4 15a1.7 1.7 0 00.3 1.8l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.8-.3 1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.1a1.7 1.7 0 00-1.1-1.5 1.7 1.7 0 00-1.8.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.8 1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.1a1.7 1.7 0 001.5-1.1 1.7 1.7 0 00-.3-1.8l-.1-.1a2 2 0 112.8-2.8l.1.1a1.7 1.7 0 001.8.3H9a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.1a1.7 1.7 0 001 1.5 1.7 1.7 0 001.8-.3l.1-.1a2 2 0 112.8 2.8l-.1.1a1.7 1.7 0 00-.3 1.8V9a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.1a1.7 1.7 0 00-1.5 1z",
    download: "M12 4v11m0 0l-4-4m4 4l4-4M5 20h14",
    refresh: "M20 11a8 8 0 10-2.3 5.7M20 4v7h-7",
    trash: "M5 7h14M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    pause: "M8 5v14M16 5v14",
    play: "M7 4l13 8-13 8z",
    print: "M7 9V3h10v6M7 17H5a2 2 0 01-2-2v-4a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2h-2M7 14h10v7H7z",
    search: "M11 19a8 8 0 100-16 8 8 0 000 16zM21 21l-4.3-4.3",
    x: "M6 6l12 12M18 6L6 18",
    camera: "M4 7h3l2-3h6l2 3h3v12H4zM12 17a4 4 0 100-8 4 4 0 000 8z",
  };
  return (
    <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d={paths[name]} />
    </svg>
  );
}

export function Spark({ data, color = "var(--accent)" }: { data: number[]; color?: string }) {
  if (data.length < 2) return <svg className="spark" viewBox="0 0 90 26" />;
  const min = Math.min(...data);
  const span = Math.max(1, Math.max(...data) - min);
  const pts = data.map((v, i) => `${((i / (data.length - 1)) * 88 + 1).toFixed(1)},${(24 - ((v - min) / span) * 22).toFixed(1)}`).join(" ");
  return (
    <svg className="spark" viewBox="0 0 90 26">
      <polyline points={pts} fill="none" stroke={color} strokeWidth="1.6" strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

export function TimeChart({
  data,
  series,
  marks = [],
  format,
  showDate,
  height = 190,
}: {
  data: object[];
  series: { key: string; color: string; name: string }[];
  marks?: { t: number; mark: string }[];
  format: (v: number) => string;
  showDate?: boolean;
  height?: number;
}) {
  const tick = (v: number) =>
    new Date(v).toLocaleString(undefined, showDate ? { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" } : { hour: "2-digit", minute: "2-digit" });
  return (
    <div style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 8, right: 12, bottom: 0, left: 0 }}>
          <CartesianGrid stroke="var(--line)" vertical={false} />
          <XAxis dataKey="t" type="number" scale="time" domain={["dataMin", "dataMax"]} tickFormatter={tick} stroke="var(--muted)" fontSize={11} tickLine={false} minTickGap={50} />
          <YAxis stroke="var(--muted)" fontSize={11} tickLine={false} axisLine={false} width={46} tickFormatter={(v) => format(Number(v))} domain={["auto", "auto"]} />
          <Tooltip
            contentStyle={{ background: "var(--tooltip)", border: "1px solid var(--line)", borderRadius: 12, fontSize: 12 }}
            labelFormatter={(v) => new Date(Number(v)).toLocaleString()}
            formatter={(v, n) => [format(Number(v)), n]}
          />
          {marks.map((m) => (
            <ReferenceLine key={m.t} x={m.t} stroke="var(--accent)" strokeDasharray="4 3" label={{ value: m.mark, fill: "var(--accent)", fontSize: 11, position: "insideTopLeft" }} />
          ))}
          {series.map((s) => (
            <Line key={s.key} type="monotone" dataKey={s.key} name={s.name} stroke={s.color} strokeWidth={2} dot={false} connectNulls isAnimationActive={false} />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
