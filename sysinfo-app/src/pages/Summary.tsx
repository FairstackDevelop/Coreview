import { useState } from "react";
import type { HardwareInfo } from "../api";
import { bytes, duration, mhz, pct } from "../format";
import { buildHtml, saveAs } from "../report";
import { useSettings } from "../settings";
import { Badge, Bar, Card, Icon, PageHead, Row } from "../ui";

export default function Summary({ hw, reload }: { hw: HardwareInfo; reload: () => void }) {
  const { t, locale, settings } = useSettings();
  const [note, setNote] = useState<string | null>(null);

  const stamp = new Date().toISOString().slice(0, 10);
  const run = async (name: string, ext: string, content: string) => {
    try {
      const path = await saveAs(name, ext, content);
      if (path) setNote(t("exp.saved", { path }));
    } catch (e) {
      setNote(`${t("err.generic")}: ${e}`);
    }
  };

  const b = hw.battery;
  const healthy = (h: string) => /verified|healthy|ok/i.test(h);

  return (
    <>
      <PageHead
        title={hw.os.hostname || t("nav.summary")}
        sub={`${hw.os.version} · ${hw.os.arch}`}
        right={
          <>
            <button className="btn" onClick={reload}>
              <Icon name="refresh" />
              {t("common.refresh")}
            </button>
            <button className="btn" onClick={() => run(`coreview-${stamp}.json`, "json", JSON.stringify(hw, null, 2))}>
              <Icon name="download" />
              {t("exp.json")}
            </button>
            <button className="btn" onClick={() => run(`coreview-${stamp}.html`, "html", buildHtml(hw, t, locale, settings.accent))}>
              <Icon name="download" />
              {t("exp.html")}
            </button>
            <button className="btn primary" onClick={() => window.print()}>
              <Icon name="print" />
              {t("exp.print")}
            </button>
          </>
        }
      />
      {note && <div className="note">{note}</div>}

      <div className="grid">
        <Card title={t("sum.cpu")} className="span2">
          <div className="hero">{hw.cpu.brand || t("common.unknown")}</div>
          <div className="cols">
            <Row label={t("sum.vendor")} value={hw.cpu.vendor} />
            <Row label={t("sum.physical")} value={hw.cpu.physicalCores} />
            <Row label={t("sum.logical")} value={hw.cpu.logicalCores} />
            <Row label={t("sum.baseClock")} value={mhz(hw.cpu.baseMhz, locale)} />
          </div>
        </Card>

        <Card title={t("sum.memory")}>
          <div className="hero">{bytes(hw.memory.total, locale)}</div>
          <Row label={t("sum.swap")} value={hw.memory.swapTotal ? bytes(hw.memory.swapTotal, locale) : undefined} />
          {hw.memory.modules.map((m, i) => (
            <div key={i} className="sub">
              <Row label={t("sum.type")} value={m.kind} />
              <Row label={t("sum.speed")} value={m.speed} />
              <Row label={t("sum.manufacturer")} value={m.manufacturer} />
              <Row label={t("common.size")} value={m.size} />
            </div>
          ))}
        </Card>

        <Card title={t("sum.os")}>
          <div className="hero">{hw.os.name}</div>
          <Row label={t("sum.version")} value={hw.os.version} />
          <Row label={t("sum.kernel")} value={hw.os.kernel} />
          <Row label={t("sum.arch")} value={hw.os.arch} />
          <Row label={t("sum.uptime")} value={duration(hw.os.uptimeSecs, locale)} />
        </Card>

        <Card title={t("sum.board")}>
          <div className="hero">{hw.board.model || hw.board.manufacturer || t("common.unknown")}</div>
          <Row label={t("sum.manufacturer")} value={hw.board.manufacturer} />
          <Row label={t("sum.serial")} value={hw.board.serial} />
          <Row label={t("sum.firmware")} value={hw.board.firmware} />
        </Card>

        {hw.gpus.map((g, i) => (
          <Card key={i} title={t("sum.graphics")}>
            <div className="hero">{g.name}</div>
            <Row label={t("sum.vendor")} value={g.vendor} />
            <Row label={t("sum.vram")} value={g.vram} />
            <Row label={t("sum.gpuCores")} value={g.cores} />
            <Row label={t("sum.driver")} value={g.driver} />
            {g.displays.map((d, j) => (
              <Row key={j} label={t("sum.displays")} value={d} />
            ))}
          </Card>
        ))}

        {b && (
          <Card title={t("sum.battery")}>
            <div className="hero">{pct(b.percent, locale)}</div>
            <Bar value={b.percent} tone={b.percent < 20 ? "danger" : undefined} />
            <Row label={t("common.status")} value={b.charging ? t("sum.charging") : b.plugged ? t("sum.plugged") : t("sum.onBattery")} />
            <Row label={t("sum.cycles")} value={b.cycleCount ?? undefined} />
            <Row label={t("sum.health")} value={b.healthPercent !== null ? pct(b.healthPercent, locale) : undefined} />
            <Row label={t("sum.capacity")} value={b.maxMah && b.designMah ? `${b.maxMah} / ${b.designMah} mAh` : undefined} />
          </Card>
        )}

        <Card title={t("sum.drives")} className="span2">
          {hw.physicalDisks.map((d, i) => (
            <div key={i} className="row">
              <span>{d.name}</span>
              <b>
                {bytes(d.size, locale)} <Badge tone={healthy(d.health) ? "ok" : "warn"}>{d.health || t("common.unknown")}</Badge>
              </b>
            </div>
          ))}
          {hw.disks
            .filter((d) => d.total > 0)
            .map((d, i) => {
              const used = ((d.total - d.available) / d.total) * 100;
              return (
                <div key={i} className="vol">
                  <div className="row">
                    <span>{d.mount}</span>
                    <b>
                      {bytes(d.total - d.available, locale)} / {bytes(d.total, locale)}
                    </b>
                  </div>
                  <Bar value={used} />
                </div>
              );
            })}
        </Card>
      </div>
    </>
  );
}
