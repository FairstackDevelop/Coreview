import { useState } from "react";
import { useLive, type MergedTemp } from "../live";
import { bytes, mhz, pct, rate } from "../format";
import type { Key } from "../i18n";
import { groupOrder, tone } from "../sensors";
import { useSettings } from "../settings";
import { Bar, Card, Chart, Icon, PageHead, Ring } from "../ui";

function TempGroups({ temps }: { temps: MergedTemp[] }) {
  const { t } = useSettings();
  const [open, setOpen] = useState(false);
  const groups = groupOrder
    .map((g) => {
      const items = temps.filter((x) => x.group === g);
      const values = items.map((x) => x.celsius);
      return {
        g,
        items,
        avg: values.reduce((a, b) => a + b, 0) / (values.length || 1),
        max: Math.max(0, ...values),
      };
    })
    .filter((x) => x.items.length > 0);

  return (
    <>
      <div className="tgroups">
        {groups.map(({ g, items, avg, max }) => (
          <div key={g} className="tgroup">
            <span className="tname">{t(`tg.${g}` as Key)}</span>
            <strong>{Math.round(avg)}°C</strong>
            <Bar value={(max / 110) * 100} tone={tone(max)} />
            <small className="muted">
              {t("mon.max")} {Math.round(max)}°C · {t("mon.sensors", { n: items.length })}
            </small>
          </div>
        ))}
      </div>
      <button className="link" onClick={() => setOpen(!open)}>
        {open ? "−" : "+"} {t("mon.showAll")}
      </button>
      {open && (
        <div className="temps">
          {[...temps]
            .sort((a, b) => a.label.localeCompare(b.label, undefined, { numeric: true }))
            .map((x, i) => (
              <div key={i} className="temp">
                <span title={x.label}>{x.label}</span>
                <Bar value={(x.celsius / 110) * 100} tone={tone(x.celsius)} />
                <b>{Math.round(x.celsius)}°C</b>
              </div>
            ))}
        </div>
      )}
    </>
  );
}

export default function Monitor() {
  const { t, locale } = useSettings();
  const { latest: s, temps, history, power, paused, setPaused } = useLive();

  if (!s) return <div className="empty">{t("common.loading")}</div>;

  const memPct = s.memTotal ? (s.memUsed / s.memTotal) * 100 : 0;
  const hottest = temps.filter((x) => x.group !== "battery").reduce((m, x) => Math.max(m, x.celsius), 0);

  return (
    <>
      <PageHead
        title={t("nav.monitor")}
        right={
          <button className="btn" onClick={() => setPaused(!paused)}>
            <Icon name={paused ? "play" : "pause"} />
            {paused ? t("mon.resume") : t("mon.pause")}
          </button>
        }
      />

      <div className="rings">
        <Card className="ring-card">
          <Ring value={s.cpuTotal} label={pct(s.cpuTotal, locale)} sub={t("mon.cpu")} />
          <small className="muted">{mhz(s.cpuMhz, locale)}</small>
        </Card>
        <Card className="ring-card">
          <Ring value={memPct} label={pct(memPct, locale)} sub={t("mon.ram")} />
          <small className="muted">
            {bytes(s.memUsed, locale)} / {bytes(s.memTotal, locale)}
          </small>
        </Card>
        <Card className="ring-card">
          <Ring value={Math.min(100, hottest)} label={hottest ? `${Math.round(hottest)}°` : "—"} sub={t("mon.temps")} />
          <small className="muted">{s.load.map((l) => l.toFixed(2)).join(" · ")}</small>
        </Card>
      </div>

      <div className="grid">
        <Card title={t("mon.cpu")} right={<b className="accent">{pct(s.cpuTotal, locale)}</b>}>
          <Chart data={history} max={100} series={[{ key: "cpu", color: "var(--accent)", name: t("mon.cpu") }]} format={(v) => pct(v, locale)} />
        </Card>
        <Card title={t("mon.ram")} right={<b className="accent">{pct(memPct, locale)}</b>}>
          <Chart data={history} max={100} series={[{ key: "mem", color: "var(--c2)", name: t("mon.ram") }]} format={(v) => pct(v, locale)} />
        </Card>
        <Card title={t("nav.network")} right={<small className="muted">↓ {rate(s.netRx, locale)} · ↑ {rate(s.netTx, locale)}</small>}>
          <Chart
            data={history}
            series={[
              { key: "rx", color: "var(--c3)", name: t("mon.down") },
              { key: "tx", color: "var(--c4)", name: t("mon.up") },
            ]}
            format={(v) => rate(v, locale)}
          />
        </Card>
        <Card title={t("nav.storage")} right={<small className="muted">R {rate(s.diskRead, locale)} · W {rate(s.diskWrite, locale)}</small>}>
          <Chart
            data={history}
            series={[
              { key: "read", color: "var(--c3)", name: t("mon.read") },
              { key: "write", color: "var(--c4)", name: t("mon.write") },
            ]}
            format={(v) => rate(v, locale)}
          />
        </Card>

        {power?.watts != null && (
          <Card title={t("pow.system")} right={<b className="accent">{power.watts.toFixed(1)} W</b>}>
            <Chart data={history} series={[{ key: "power", color: "var(--c4)", name: t("pow.system") }]} format={(v) => `${v.toFixed(1)} W`} />
          </Card>
        )}

        <Card title={t("mon.cores")} className="span2">
          <div className="cores">
            {s.cpuCores.map((c, i) => (
              <div key={i} className="core">
                <span>{i + 1}</span>
                <Bar value={c} />
                <em>{Math.round(c)}</em>
              </div>
            ))}
          </div>
        </Card>

        <Card title={t("mon.temps")} className="span2">
          {temps.length === 0 ? (
            <p className="muted">{t("mon.noSensors")}</p>
          ) : (
            <TempGroups temps={temps} />
          )}
        </Card>
      </div>
    </>
  );
}
