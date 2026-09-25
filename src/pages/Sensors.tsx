import { useEffect, useMemo, useState } from "react";
import { useLive, type SensorStat } from "../live";
import { duration, pct } from "../format";
import type { Key } from "../i18n";
import { describe, describeSmc, groupOf, groupOrder, smcGroup, tone, type Group } from "../sensors";
import { useSettings } from "../settings";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type SensorStatus } from "../api";
import { Bar, Card, Chart, Icon, PageHead, Spark } from "../ui";

interface Row {
  key: string;
  raw: string;
  name: string;
  group: Group;
  stat: SensorStat;
}

function SensorAccess() {
  const { t } = useSettings();
  const [status, setStatus] = useState<SensorStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    let alive = true;
    const load = () => api.sensorStatus().then((s) => alive && setStatus(s)).catch(() => {});
    load();
    const id = setInterval(load, 6000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  if (!status || status.platform !== "windows") return null;

  const install = async () => {
    setBusy(true);
    setResult(null);
    try {
      await api.installDriver();
      setResult({ ok: true, text: t("sd.installed") });
    } catch (e) {
      setResult({ ok: false, text: `${t("sd.failed")}: ${e}` });
    }
    setBusy(false);
    api.sensorStatus().then(setStatus).catch(() => {});
  };

  return (
    <>
      {status.helper !== "running" && (
        <div className="note error">
          {t("sd.helperError")}
          {status.error ? `: ${status.error}` : ""}
        </div>
      )}
      {!status.admin && status.helper === "running" && <div className="note error">{t("sd.admin")}</div>}
      {!status.driver && (
        <Card title={t("sd.driverTitle")} className="driver-card">
          <p>{t("sd.driverText")}</p>
          <div className="page-actions">
            <button className="btn primary" onClick={install} disabled={busy}>
              <Icon name="download" />
              {busy ? t("sd.installing") : t("sd.install")}
            </button>
            <button className="btn" onClick={() => openUrl("https://pawnio.eu")}>
              {t("sd.more")}
            </button>
          </div>
          {result && <div className={`note ${result.ok ? "" : "error"}`}>{result.text}</div>}
        </Card>
      )}
    </>
  );
}

export default function Sensors() {
  const { t, locale } = useSettings();
  const { history, power, smc, fanHist, sensors, resetSensors } = useLive();
  const [query, setQuery] = useState("");
  const [showAll, setShowAll] = useState(false);

  const num = (v: number | null | undefined, unit: string, digits = 1) =>
    v === null || v === undefined ? "—" : `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(v)} ${unit}`;

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    const all: Row[] = Object.entries(sensors).map(([key, stat]) => {
      if (key.startsWith("smc:")) {
        const id = key.slice(4);
        const m = smc?.temps.find((x) => x.key === id);
        return { key, raw: m?.hw ? m.hw : `SMC ${id}`, name: m ? describeSmc(m, t) : id, group: m ? smcGroup(m.group) : "other", stat };
      }
      return { key, raw: key, name: describe(key, t), group: groupOf(key), stat };
    });
    return groupOrder
      .map((g) => ({
        g,
        items: all
          .filter((r) => r.group === g && (!q || `${r.raw} ${r.name}`.toLowerCase().includes(q)))
          .sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }) || a.raw.localeCompare(b.raw)),
      }))
      .filter((x) => x.items.length > 0 && (x.g !== "other" || showAll || q));
  }, [sensors, smc, query, showAll, t]);

  const hiddenOther = useMemo(() => Object.keys(sensors).filter((k) => groupOf(k) === "other" || (k.startsWith("smc:") && smcGroup(smc?.temps.find((x) => x.key === k.slice(4))?.group ?? "other") === "other")).length, [sensors, smc]);

  const deg = (v: number) => `${v.toFixed(1)}°`;
  const parts = power
    ? ([
        ["pw.cpu", power.cpuWatts],
        ["pw.gpu", power.gpuWatts],
        ["pw.ane", power.aneWatts],
        ["pw.dram", power.dramWatts],
      ] as [Key, number | null][]).filter((x): x is [Key, number] => x[1] !== null)
    : [];
  const partMax = Math.max(0.1, ...parts.map((x) => x[1]));
  const battW = power?.batteryWatts ?? null;

  return (
    <>
      <PageHead
        title={t("nav.sensors")}
        right={
          <button className="btn" onClick={resetSensors}>
            <Icon name="refresh" />
            {t("sn.reset")}
          </button>
        }
      />

      <SensorAccess />

      <h2 className="section first">{t("pow.title")}</h2>
      {!power ? (
        <Card>
          <p className="muted">{t("pow.unavailable")}</p>
        </Card>
      ) : (
        <div className="grid">
          <Card title={t("pow.system")} right={<b className="accent">{num(power.watts, "W")}</b>}>
            <Chart data={history} series={[{ key: "power", color: "var(--c4)", name: t("pow.system") }]} format={(v) => num(v, "W")} height={120} />
          </Card>

          {parts.length > 0 && (
            <Card title={t("pow.title")}>
              {parts.map(([k, w]) => (
                <div key={k} className="meter">
                  <span>{t(k)}</span>
                  <Bar value={(w / partMax) * 100} />
                  <b>{num(w, "W", 2)}</b>
                </div>
              ))}
            </Card>
          )}

          {power.gpuLoad !== null && (
            <Card title={t("mon.gpu")} right={<b className="accent">{pct(power.gpuLoad, locale)}</b>}>
              <Chart data={history} max={100} series={[{ key: "gpu", color: "var(--c2)", name: t("mon.gpu") }]} format={(v) => pct(v, locale)} height={100} />
            </Card>
          )}

          <Card title={t("pow.battery")} right={power.percent !== null ? <b className="accent">{pct(power.percent, locale)}</b> : undefined}>
            {power.percent !== null && <Bar value={power.percent} tone={power.percent < 20 ? "danger" : undefined} />}
            <div className="row">
              <span>{t("common.status")}</span>
              <b>{power.charging ? t("pow.charging") : power.onAc ? t("sum.plugged") : t("pow.discharging")}</b>
            </div>
            <div className="row">
              <span>{t("pow.title")}</span>
              <b>{battW === null ? "—" : `${battW > 0 ? "+" : "−"}${num(Math.abs(battW), "W")}`}</b>
            </div>
            {power.batteryVolts !== null && (
              <div className="row">
                <span>{t("pow.voltage")}</span>
                <b>{num(power.batteryVolts, "V", 2)}</b>
              </div>
            )}
            {power.batteryAmps !== null && (
              <div className="row">
                <span>{t("pow.current")}</span>
                <b>{num(power.batteryAmps, "A", 2)}</b>
              </div>
            )}
            {power.minutesRemaining !== null && !power.charging && (
              <div className="row">
                <span>{t("pow.remaining")}</span>
                <b>{duration(power.minutesRemaining * 60, locale)}</b>
              </div>
            )}
          </Card>

          <Card title={t("pow.adapter")}>
            <div className="hero">{power.onAc ? num(power.adapterWatts, "W") : "—"}</div>
            {power.adapterRated ? (
              <div className="row">
                <span>{t("pow.rated", { w: power.adapterRated })}</span>
                <b />
              </div>
            ) : null}
          </Card>
        </div>
      )}

      <h2 className="section">{t("fan.title")}</h2>
      {smc && smc.fans.length > 0 ? (
        <div className="grid">
          {smc.fans.map((f) => (
            <Card key={f.id} title={f.name || t("fan.name", { n: f.id + 1 })} right={<b className="accent">{Math.round(f.rpm)} RPM</b>}>
              {f.max > 0 && <Bar value={(f.rpm / f.max) * 100} />}
              <Spark data={fanHist[f.id] ?? []} color="var(--c3)" />
              {f.max > 0 && (
                <div className="row">
                  <span>
                    {t("sn.min")} / {t("sn.max")}
                  </span>
                  <b>
                    {Math.round(f.min)} / {Math.round(f.max)} RPM
                  </b>
                </div>
              )}
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <p className="muted">{t("fan.none")}</p>
        </Card>
      )}

      <h2 className="section">{t("mon.temps")}</h2>
      <Card>
        <div className="search">
          <Icon name="search" />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("common.search")} />
        </div>
        {rows.length === 0 ? (
          <p className="muted">{t("mon.noSensors")}</p>
        ) : (
          <div className="table-wrap tall">
            <table>
              <thead>
                <tr>
                  <th>{t("sn.sensor")}</th>
                  <th>{t("sn.now")}</th>
                  <th />
                  <th>{t("sn.min")}</th>
                  <th>{t("sn.avg")}</th>
                  <th>{t("sn.max")}</th>
                  <th>{t("sn.trend")}</th>
                </tr>
              </thead>
              {rows.map(({ g, items }) => (
                <tbody key={g}>
                  <tr className="group-row">
                    <td colSpan={7}>
                      {t(`tg.${g}` as Key)} · {items.length}
                    </td>
                  </tr>
                  {items.map((r) => (
                    <tr key={r.key}>
                      <td>
                        {r.name}
                        {r.name !== r.raw && <small className="raw">{r.raw}</small>}
                      </td>
                      <td>
                        <b className={`temp-now ${tone(r.stat.now) ?? ""}`}>{deg(r.stat.now)}</b>
                      </td>
                      <td className="bar-cell">
                        <Bar value={(r.stat.now / 110) * 100} tone={tone(r.stat.now)} />
                      </td>
                      <td>{deg(r.stat.min)}</td>
                      <td>{deg(r.stat.sum / r.stat.n)}</td>
                      <td>{deg(r.stat.max)}</td>
                      <td>
                        <Spark data={r.stat.hist} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              ))}
            </table>
          </div>
        )}
        {hiddenOther > 0 && !query && (
          <button className="link" onClick={() => setShowAll(!showAll)}>
            {showAll ? "−" : "+"} {t("mon.showAll")} ({hiddenOther})
          </button>
        )}
      </Card>
    </>
  );
}
