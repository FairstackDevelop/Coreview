import type { HardwareInfo } from "../api";
import { bytes } from "../format";
import { useSettings } from "../settings";
import { Badge, Bar, Card, PageHead } from "../ui";

export default function Storage({ hw }: { hw: HardwareInfo }) {
  const { t, locale } = useSettings();
  const healthy = (h: string) => /verified|healthy|ok/i.test(h);

  return (
    <>
      <PageHead title={t("nav.storage")} />
      <div className="grid">
        {hw.physicalDisks.map((d, i) => (
          <Card key={i} title={t("sto.physical")} right={<Badge tone={healthy(d.health) ? "ok" : d.health ? "warn" : "muted"}>{d.health || t("common.unknown")}</Badge>}>
            <div className="hero">{d.name}</div>
            <div className="row"><span>{t("common.size")}</span><b>{bytes(d.size, locale)}</b></div>
            {d.media && <div className="row"><span>{t("sto.media")}</span><b>{d.media}</b></div>}
            {d.protocol && <div className="row"><span>{t("sto.protocol")}</span><b>{d.protocol}</b></div>}
            <div className="row"><span>{t("sto.smart")}</span><b>{d.health || "—"}</b></div>
          </Card>
        ))}
      </div>

      <h2 className="section">{t("sto.volumes")}</h2>
      <div className="grid">
        {hw.disks
          .filter((d) => d.total > 0)
          .map((d, i) => {
            const used = d.total - d.available;
            return (
              <Card key={i}>
                <div className="hero">{d.name || d.mount}</div>
                <Bar value={(used / d.total) * 100} />
                <div className="row"><span>{t("common.used")}</span><b>{bytes(used, locale)}</b></div>
                <div className="row"><span>{t("common.free")}</span><b>{bytes(d.available, locale)}</b></div>
                <div className="row"><span>{t("common.total")}</span><b>{bytes(d.total, locale)}</b></div>
                <div className="row"><span>{t("sto.mount")}</span><b>{d.mount}</b></div>
                <div className="row"><span>{t("sto.fs")}</span><b>{d.fs}</b></div>
                <div className="row"><span>{t("common.status")}</span><b>{d.kind}{d.removable ? ` · ${t("sto.removable")}` : ""}</b></div>
              </Card>
            );
          })}
      </div>
    </>
  );
}
