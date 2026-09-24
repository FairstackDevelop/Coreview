import { useEffect, useMemo, useState } from "react";
import { api, type Connection, type HardwareInfo } from "../api";
import { useLive } from "../live";
import { rate } from "../format";
import { useSettings } from "../settings";
import { Badge, Card, Chart, Icon, PageHead, Row } from "../ui";

export default function Network({ hw }: { hw: HardwareInfo }) {
  const { t, locale } = useSettings();
  const { latest, history } = useLive();
  const [conns, setConns] = useState<Connection[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    let alive = true;
    const load = () => api.connections().then((c) => alive && setConns(c)).catch(() => {});
    load();
    const id = setInterval(load, 5000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? conns.filter((c) => `${c.process} ${c.pid} ${c.local} ${c.remote} ${c.state}`.toLowerCase().includes(q)) : conns;
  }, [conns, query]);

  return (
    <>
      <PageHead title={t("nav.network")} sub={latest ? `↓ ${rate(latest.netRx, locale)} · ↑ ${rate(latest.netTx, locale)}` : undefined} />
      <Card>
        <Chart
          data={history}
          series={[
            { key: "rx", color: "var(--c3)", name: t("mon.down") },
            { key: "tx", color: "var(--c4)", name: t("mon.up") },
          ]}
          format={(v) => rate(v, locale)}
        />
      </Card>

      <h2 className="section">{t("net.adapters")}</h2>
      <div className="grid">
        {hw.network
          .filter((n) => n.ips.length > 0 || n.mac !== "00:00:00:00:00:00")
          .map((n) => (
            <Card key={n.name} title={n.name} right={<Badge tone={/up/i.test(n.state) ? "ok" : "muted"}>{n.state}</Badge>}>
              <Row label={t("net.mac")} value={n.mac} />
              <Row label={t("net.mtu")} value={n.mtu} />
              {n.ips.map((ip) => (
                <Row key={ip} label={t("net.ip")} value={ip} />
              ))}
            </Card>
          ))}
      </div>

      <h2 className="section">{t("net.connections")}</h2>
      <Card>
        <div className="search">
          <Icon name="search" />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("common.search")} />
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("net.process")}</th>
                <th>{t("proc.pid")}</th>
                <th>{t("net.local")}</th>
                <th>{t("net.remote")}</th>
                <th>{t("net.state")}</th>
              </tr>
            </thead>
            <tbody>
              {shown.map((c, i) => (
                <tr key={i}>
                  <td>{c.process || "—"}</td>
                  <td className="mono">{c.pid}</td>
                  <td className="mono">{c.local}</td>
                  <td className="mono">{c.remote}</td>
                  <td>
                    <Badge tone={c.state === "ESTABLISHED" ? "ok" : "muted"}>{c.state}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </>
  );
}
