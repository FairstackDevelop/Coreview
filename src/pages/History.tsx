import { useCallback, useEffect, useMemo, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { api, type HistoryData } from "../api";
import { mhz, pct } from "../format";
import { saveAs } from "../report";
import { useSettings } from "../settings";
import { Card, Empty, Icon, PageHead, TimeChart } from "../ui";

type Range = "1h" | "6h" | "24h" | "7d" | "custom";

const spans: Record<Exclude<Range, "custom">, number> = { "1h": 3_600_000, "6h": 21_600_000, "24h": 86_400_000, "7d": 604_800_000 };

const toLocalInput = (ms: number) => {
  const d = new Date(ms - new Date().getTimezoneOffset() * 60_000);
  return d.toISOString().slice(0, 16);
};

export default function History() {
  const { t, locale } = useSettings();
  const [range, setRange] = useState<Range>("1h");
  const [from, setFrom] = useState(() => toLocalInput(Date.now() - spans["6h"]));
  const [to, setTo] = useState(() => toLocalInput(Date.now()));
  const [data, setData] = useState<HistoryData | null>(null);
  const [name, setName] = useState("");
  const [note, setNote] = useState<string | null>(null);

  const bounds = useCallback((): [number, number] => {
    if (range === "custom") return [new Date(from).getTime(), new Date(to).getTime()];
    const now = Date.now();
    return [now - spans[range], now];
  }, [range, from, to]);

  const load = useCallback(() => {
    const [a, b] = bounds();
    if (!(b > a)) return;
    api.historyQuery(a, b, 600).then(setData).catch(() => setData({ points: [], marks: [] }));
  }, [bounds]);

  useEffect(() => {
    load();
    if (range === "custom") return;
    const id = setInterval(load, 15_000);
    return () => clearInterval(id);
  }, [load, range]);

  const fmt = {
    temp: (v: number) => `${v.toFixed(1)}°C`,
    freq: (v: number) => mhz(Math.round(v), locale),
    pct: (v: number) => pct(v, locale),
    watt: (v: number) => `${v.toFixed(1)} W`,
    rpm: (v: number) => `${Math.round(v)} RPM`,
  };

  const groups = useMemo(
    () => [
      { id: "temp", title: t("mon.temps"), fmt: fmt.temp, series: [
        { key: "tCpu", name: t("hi.tCpu"), color: "var(--danger)" },
        { key: "tGpu", name: t("hi.tGpu"), color: "var(--c2)" },
        { key: "tMax", name: t("hi.tMax"), color: "var(--warn)" },
      ] },
      { id: "freq", title: t("mon.freq"), fmt: fmt.freq, series: [{ key: "freq", name: t("mon.freq"), color: "var(--c3)" }] },
      { id: "load", title: t("mon.cpu"), fmt: fmt.pct, series: [
        { key: "cpu", name: "CPU", color: "var(--accent)" },
        { key: "gpu", name: t("mon.gpu"), color: "var(--c2)" },
      ] },
      { id: "power", title: t("pow.system"), fmt: fmt.watt, series: [
        { key: "power", name: t("pow.system"), color: "var(--c4)" },
        { key: "cpuW", name: "CPU", color: "var(--accent)" },
        { key: "gpuW", name: "GPU", color: "var(--c2)" },
      ] },
      { id: "fan", title: t("fan.title"), fmt: fmt.rpm, series: [{ key: "fan", name: t("fan.speed"), color: "var(--c3)" }] },
      { id: "mem", title: t("mon.ram"), fmt: fmt.pct, series: [{ key: "mem", name: t("mon.ram"), color: "var(--c4)" }] },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [t, locale],
  );

  const stats = (key: string) => {
    const values = (data?.points ?? []).map((p) => p[key]).filter((v): v is number => typeof v === "number");
    if (!values.length) return null;
    return { min: Math.min(...values), max: Math.max(...values), avg: values.reduce((a, b) => a + b, 0) / values.length };
  };

  const [a, b] = bounds();
  const showDate = b - a > 86_400_000;

  const addMark = async () => {
    await api.historyAppend({ t: Date.now(), mark: name.trim() || new Date().toLocaleTimeString(locale) });
    setName("");
    load();
  };

  const exportCsv = async () => {
    try {
      const path = await saveAs(`coreview-history.csv`, "csv", await api.historyCsv(a, b));
      if (path) setNote(t("exp.saved", { path }));
    } catch (e) {
      setNote(String(e));
    }
  };

  const clear = async () => {
    if (await ask(t("hi.clearAsk"), { title: "Fairstack Coreview", kind: "warning" })) {
      await api.historyClear();
      load();
    }
  };

  const empty = !data || data.points.length === 0;

  return (
    <>
      <PageHead
        title={t("nav.history")}
        right={
          <>
            <button className="btn" onClick={exportCsv} disabled={empty}>
              <Icon name="download" />
              {t("hi.csv")}
            </button>
            <button className="btn" onClick={clear}>
              <Icon name="trash" />
              {t("hi.clear")}
            </button>
          </>
        }
      />
      {note && <div className="note">{note}</div>}

      <Card>
        <div className="history-controls">
          <div className="langs">
            {(["1h", "6h", "24h", "7d", "custom"] as Range[]).map((r) => (
              <button key={r} className={`chip ${range === r ? "active" : ""}`} onClick={() => setRange(r)}>
                {t(`hi.${r === "custom" ? "custom" : "r" + r}` as never)}
              </button>
            ))}
          </div>
          {range === "custom" && (
            <div className="dates">
              <label>
                {t("hi.from")}
                <input type="datetime-local" value={from} onChange={(e) => setFrom(e.target.value)} />
              </label>
              <label>
                {t("hi.to")}
                <input type="datetime-local" value={to} onChange={(e) => setTo(e.target.value)} />
              </label>
            </div>
          )}
          <div className="inline-form">
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t("hi.markName")} onKeyDown={(e) => e.key === "Enter" && addMark()} />
            <button className="btn primary" onClick={addMark}>
              <Icon name="plus" />
              {t("hi.mark")}
            </button>
          </div>
        </div>
      </Card>

      {empty ? (
        <Empty>{t("hi.empty")}</Empty>
      ) : (
        <div className="grid">
          {groups
            .filter((g) => g.series.some((s) => stats(s.key)))
            .map((g) => (
              <Card key={g.id} title={g.title} className="span2">
                <TimeChart
                  data={data!.points}
                  marks={data!.marks}
                  showDate={showDate}
                  format={g.fmt}
                  series={g.series.filter((s) => stats(s.key))}
                />
                <div className="stat-table">
                  {g.series
                    .filter((s) => stats(s.key))
                    .map((s) => {
                      const st = stats(s.key)!;
                      return (
                        <div key={s.key} className="stat-line">
                          <span className="dot" style={{ background: s.color }} />
                          <em>{s.name}</em>
                          <small>
                            {t("sn.min")} {g.fmt(st.min)} · {t("sn.avg")} {g.fmt(st.avg)} · {t("sn.max")} {g.fmt(st.max)}
                          </small>
                        </div>
                      );
                    })}
                </div>
              </Card>
            ))}
        </div>
      )}
    </>
  );
}
