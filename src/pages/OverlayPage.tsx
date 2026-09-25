import { useEffect, useState } from "react";
import { api, type FpsStatus } from "../api";
import { type OverlayConfig, useOverlayConfig } from "../overlayConfig";
import type { Key } from "../i18n";
import { useSettings } from "../settings";
import { Card, Field, Icon, PageHead, Segmented, Slider, Toggle } from "../ui";

const corners: { value: OverlayConfig["corner"]; key: Key }[] = [
  { value: "tl", key: "ov.tl" },
  { value: "tr", key: "ov.tr" },
  { value: "bl", key: "ov.bl" },
  { value: "br", key: "ov.br" },
];

const metrics: { id: keyof OverlayConfig["show"]; label: Key }[] = [
  { id: "fps", label: "ov.fps" },
  { id: "low", label: "ov.low" },
  { id: "frame", label: "ov.frame" },
  { id: "cpuLoad", label: "mon.cpu" },
  { id: "cpuTemp", label: "ov.cpuTemp" },
  { id: "cpuMhz", label: "mon.freq" },
  { id: "gpuLoad", label: "mon.gpu" },
  { id: "gpuTemp", label: "ov.gpuTemp" },
  { id: "ram", label: "mon.ram" },
  { id: "power", label: "pow.system" },
  { id: "fan", label: "fan.speed" },
];

export default function OverlayPage() {
  const { t } = useSettings();
  const [cfg, update] = useOverlayConfig();
  const [locked, setLocked] = useState(true);
  const [fps, setFps] = useState<FpsStatus | null>(null);

  useEffect(() => {
    api.fpsStatus().then(setFps).catch(() => {});
  }, []);

  const show = (next: OverlayConfig, isLocked = locked) => {
    if (next.enabled) api.overlayShow(next.corner, next.pos?.x ?? null, next.pos?.y ?? null, isLocked).catch(() => {});
    else api.overlayHide().catch(() => {});
  };

  const apply = (patch: Partial<OverlayConfig>) => {
    update(patch);
    show({ ...cfg, ...patch });
  };

  const toggleLock = () => {
    const next = !locked;
    setLocked(next);
    api.overlayLock(next).catch(() => {});
  };

  return (
    <>
      <PageHead
        title={t("nav.overlay")}
        right={
          <button className="btn" onClick={toggleLock} disabled={!cfg.enabled}>
            <Icon name={locked ? "pause" : "play"} />
            {locked ? t("ov.unlock") : t("ov.lock")}
          </button>
        }
      />
      <div className="grid">
        <Card>
          <Field label={t("ov.enable")}>
            <Toggle checked={cfg.enabled} onChange={(enabled) => apply({ enabled })} />
          </Field>
          <Field label={t("ov.position")}>
            <Segmented value={cfg.corner} onChange={(corner) => apply({ corner, pos: null })} options={corners.map((c) => ({ value: c.value, label: t(c.key) }))} />
          </Field>
          <Field label={t("ov.layout")}>
            <Segmented
              value={cfg.layout}
              onChange={(layout) => update({ layout })}
              options={[
                { value: "vertical", label: t("ov.vertical") },
                { value: "horizontal", label: t("ov.horizontal") },
              ]}
            />
          </Field>
          <Field label={t("ov.scale")}>
            <Slider value={cfg.scale} min={70} max={170} step={5} suffix="%" onChange={(scale) => update({ scale })} />
          </Field>
          <Field label={t("ov.opacity")}>
            <Slider value={cfg.opacity} min={0} max={95} step={5} suffix="%" onChange={(opacity) => update({ opacity })} />
          </Field>
          <p className="muted small">{t("ov.hint")}</p>
        </Card>

        <Card title={t("ov.show")}>
          {metrics.map((m) => (
            <Field key={m.id} label={t(m.label)}>
              <Toggle checked={cfg.show[m.id]} onChange={(v) => update({ show: { ...cfg.show, [m.id]: v } })} />
            </Field>
          ))}
          <p className="muted small">{t("ov.fpsNote")}</p>
          {fps && fps.error && <div className="note error">{fps.error}</div>}
          <p className="muted small">{t("ov.gameNote")}</p>
        </Card>
      </div>
    </>
  );
}
