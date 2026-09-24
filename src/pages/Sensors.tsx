import { useMemo, useState } from "react";
import { useLive } from "../live";
import { duration, pct } from "../format";
import type { Key } from "../i18n";
import { describe, groupOf, groupOrder, tone } from "../sensors";
import { useSettings } from "../settings";
import { Bar, Card, Chart, Icon, PageHead, Spark } from "../ui";

const watts = (v: number | null | undefined, locale: string) =>
  v === null || v === undefined ? "—" : `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(v)} W`;

export default function Sensors() {
  const { t, locale } = useSettings();
  const { history, power, sensors, resetSensors } = useLive();
  const [query, setQuery] = useState("");

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    return groupOrder
      .map((g) => ({
        g,
        items: Object.entries(sensors)
          .filter(([label]) => groupOf(label) === g && (!q || `${label} ${describe(label, t)}`.toLowerCase().includes(q)))
          .sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true })),
      }))
      .filter((x) => x.items.length > 0);
  }, [sensors, query, t]);

  const deg = (v: number) => `${v.toFixed(1)}°`;
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

      <h2 className="section first">{t("pow.title")}</h2>
      {!power ? (
        <Card>
          <p className="muted">{t("pow.unavailable")}</p>
        </Card>
      ) : (
        <div className="grid">
          <Card title={t("pow.system")} right={<b className="accent">{watts(power.watts, locale)}</b>}>
            <Chart data={history} series={[{ key: "power", color: "var(--c4)", name: t("pow.system") }]} format={(v) => watts(v, locale)} height={120} />
          </Card>
          <Card title={t("pow.battery")} right={power.percent !== null ? <b className="accent">{pct(power.percent, locale)}</b> : undefined}>
            {power.percent !== null && <Bar value={power.percent} tone={power.percent < 20 ? "danger" : undefined} />}
            <div className="row">
              <span>{t("common.status")}</span>
              <b>{power.charging ? t("pow.charging") : power.onAc ? t("sum.plugged") : t("pow.discharging")}</b>
            </div>
            <div className="row">
              <span>{t("pow.system")}</span>
              <b>{battW === null ? "—" : `${battW > 0 ? "+" : "−"}${watts(Math.abs(battW), locale)}`}</b>
            </div>
            {power.minutesRemaining !== null && !power.charging && (
              <div className="row">
                <span>{t("pow.remaining")}</span>
                <b>{duration(power.minutesRemaining * 60, locale)}</b>
              </div>
            )}
          </Card>
          <Card title={t("pow.adapter")}>
            <div className="hero">{power.onAc ? watts(power.adapterWatts, locale) : "—"}</div>
            {power.adapterRated && <div className="row"><span>{t("pow.rated", { w: power.adapterRated })}</span><b /></div>}
          </Card>
        </div>
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
                  {items.map(([label, s]) => (
                    <tr key={label}>
                      <td>
                        {describe(label, t)}
                        {describe(label, t) !== label && <small className="raw">{label}</small>}
                      </td>
                      <td>
                        <b className={`temp-now ${tone(s.now) ?? ""}`}>{deg(s.now)}</b>
                      </td>
                      <td className="bar-cell">
                        <Bar value={(s.now / 110) * 100} tone={tone(s.now)} />
                      </td>
                      <td>{deg(s.min)}</td>
                      <td>{deg(s.sum / s.n)}</td>
                      <td>{deg(s.max)}</td>
                      <td>
                        <Spark data={s.hist} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              ))}
            </table>
          </div>
        )}
      </Card>
    </>
  );
}
