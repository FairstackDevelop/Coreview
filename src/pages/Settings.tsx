import { languages, type Lang } from "../i18n";
import { presets, useSettings } from "../settings";
import { Card, Field, PageHead, Segmented, Slider, Toggle } from "../ui";

const swatches = ["#7c8cff", "#22d3ee", "#34d399", "#a3e635", "#fbbf24", "#f97316", "#f43f5e", "#e879f9"];

export default function Settings() {
  const { settings: s, set, reset, t } = useSettings();
  const isWindows = navigator.userAgent.includes("Windows");

  return (
    <>
      <PageHead
        title={t("nav.settings")}
        right={
          <button className="btn" onClick={reset}>
            {t("set.reset")}
          </button>
        }
      />
      <div className="grid">
        <Card title={t("set.language")}>
          <div className="langs">
            {languages.map((l) => (
              <button key={l.code} className={`chip ${s.lang === l.code ? "active" : ""}`} onClick={() => set({ lang: l.code as Lang })}>
                {l.name}
              </button>
            ))}
          </div>
        </Card>

        <Card title={t("set.presets")}>
          <div className="presets">
            {(["glass", "midnight", "paper"] as const).map((k) => (
              <button key={k} className={`preset ${k}`} onClick={() => set(presets[k])}>
                <i style={{ background: presets[k].accent }} />
                {t(k === "glass" ? "set.presetGlass" : k === "midnight" ? "set.presetMidnight" : "set.presetPaper")}
              </button>
            ))}
          </div>
        </Card>

        <Card title={t("set.appearance")}>
          <Field label={t("set.theme")}>
            <Segmented
              value={s.theme}
              onChange={(theme) => set({ theme })}
              options={[
                { value: "dark", label: t("set.dark") },
                { value: "light", label: t("set.light") },
                { value: "system", label: t("set.system") },
              ]}
            />
          </Field>
          <Field label={t("set.accent")}>
            <div className="swatches">
              {swatches.map((c) => (
                <button key={c} className={s.accent === c ? "active" : ""} style={{ background: c }} onClick={() => set({ accent: c })} aria-label={c} />
              ))}
              <input type="color" value={s.accent} onChange={(e) => set({ accent: e.target.value })} />
            </div>
          </Field>
          <Field label={t("set.radius")}>
            <Slider value={s.radius} min={4} max={28} suffix="px" onChange={(radius) => set({ radius })} />
          </Field>
          <Field label={t("set.font")}>
            <Slider value={s.font} min={85} max={125} step={5} suffix="%" onChange={(font) => set({ font })} />
          </Field>
          <Field label={t("set.density")}>
            <Segmented
              value={s.density}
              onChange={(density) => set({ density })}
              options={[
                { value: "compact", label: t("set.compact") },
                { value: "comfortable", label: t("set.comfortable") },
              ]}
            />
          </Field>
          <Field label={t("set.motion")}>
            <Toggle checked={s.motion} onChange={(motion) => set({ motion })} />
          </Field>
          <Field label={t("set.glow")}>
            <Toggle checked={s.glow} onChange={(glow) => set({ glow })} />
          </Field>
        </Card>

        <Card title={t("set.window")}>
          <Field label={t("set.effect")}>
            <Segmented
              value={s.effect}
              onChange={(effect) => set({ effect })}
              options={[
                { value: "none", label: t("set.effectNone") },
                { value: "blur", label: t("set.effectBlur") },
                ...(isWindows ? [{ value: "mica" as const, label: t("set.effectMica") }] : []),
              ]}
            />
          </Field>
          <Field label={t("set.opacity")}>
            <Slider value={s.opacity} min={10} max={100} suffix="%" onChange={(opacity) => set({ opacity })} />
          </Field>
          <Field label={t("set.blur")}>
            <Slider value={s.blur} min={0} max={60} suffix="px" onChange={(blur) => set({ blur })} />
          </Field>
        </Card>

        <Card title={t("set.monitoring")}>
          <Field label={t("set.interval")}>
            <Slider value={s.interval / 1000} min={0.5} max={5} step={0.5} suffix={` ${t("set.seconds")}`} onChange={(v) => set({ interval: v * 1000 })} />
          </Field>
          <Field label={t("set.history")}>
            <Slider value={s.history} min={30} max={300} step={10} suffix={` ${t("set.points")}`} onChange={(history) => set({ history })} />
          </Field>
        </Card>

        <Card title={t("set.alerts")}>
          <Field label={t("set.alertOn")}>
            <Toggle checked={s.alertOn} onChange={(alertOn) => set({ alertOn })} />
          </Field>
          <Field label={t("set.alertTemp")}>
            <Slider value={s.alertTemp} min={50} max={110} suffix="°C" onChange={(alertTemp) => set({ alertTemp })} />
          </Field>
        </Card>
      </div>
    </>
  );
}
