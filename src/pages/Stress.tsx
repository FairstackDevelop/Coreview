import { useEffect, useRef, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { api, type StressStatus } from "../api";
import { duration, mhz, pct } from "../format";
import { useLive } from "../live";
import { groupOf } from "../sensors";
import { useSettings } from "../settings";
import { Badge, Bar, Card, Chart, Field, Icon, PageHead, Segmented, Slider } from "../ui";

type Kind = "cpu" | "memory" | "disk" | "gpu" | "all" | "full";

interface Sample {
  t: number;
  cpu: number;
  temp: number;
  freq: number;
  power: number | null;
  gpu: number | null;
  rate: number;
}

interface Result {
  reason: string;
  kind: Kind;
  seconds: number;
  avgTemp: number;
  maxTemp: number;
  freqChange: number;
  throttled: boolean;
  avgPower: number | null;
  peakPower: number | null;
  score: number;
  write: number;
  read: number;
  device: string;
}

const cache: { samples: Sample[]; result: Result | null } = { samples: [], result: null };

const mean = (a: number[]) => (a.length ? a.reduce((x, y) => x + y, 0) / a.length : 0);

function summarize(samples: Sample[], s: StressStatus): Result {
  const temps = samples.map((x) => x.temp).filter((x) => x > 0);
  const edge = Math.min(5, Math.floor(samples.length / 2));
  const first = mean(samples.slice(0, edge).map((x) => x.freq));
  const last = mean(samples.slice(-edge).map((x) => x.freq));
  const powers = samples.map((x) => x.power).filter((x): x is number => x !== null);
  const change = first > 0 ? ((last - first) / first) * 100 : 0;
  return {
    reason: s.reason,
    kind: s.kind as Kind,
    seconds: s.elapsedSecs,
    avgTemp: mean(temps),
    maxTemp: Math.max(0, ...temps),
    freqChange: change,
    throttled: s.kind !== "disk" && change < -10,
    avgPower: powers.length ? mean(powers) : null,
    peakPower: powers.length ? Math.max(...powers) : null,
    score: s.elapsedSecs > 0 ? s.ops / s.elapsedSecs : 0,
    write: s.diskWriteMbs,
    read: s.diskReadMbs,
    device: s.device,
  };
}

export default function Stress() {
  const { t, locale, settings, set } = useSettings();
  const { latest } = useLive();
  const cores = latest?.cpuCores.length ?? 4;
  const [kind, setKind] = useState<Kind>("cpu");
  const [threads, setThreads] = useState(cores);
  const [seconds, setSeconds] = useState(60);
  const [status, setStatus] = useState<StressStatus | null>(null);
  const [samples, setSamples] = useState<Sample[]>(cache.samples);
  const [result, setResult] = useState<Result | null>(cache.result);
  const running = status?.running ?? false;
  const latestRef = useRef(latest);
  latestRef.current = latest;
  const powerRef = useRef<number | null>(null);
  const { power } = useLive();
  powerRef.current = power?.watts ?? null;
  const gpuRef = useRef<number | null>(null);
  gpuRef.current = power?.gpuLoad ?? null;
  const wasRunning = useRef(false);
  const samplesRef = useRef(samples);
  samplesRef.current = samples;

  useEffect(() => {
    cache.samples = samples;
    cache.result = result;
  }, [samples, result]);

  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const s = await api.stressStatus();
        if (!alive) return;
        setStatus(s);
        if (s.running) {
          wasRunning.current = true;
          const live = latestRef.current;
          const hottest = Math.max(0, ...(live?.temps ?? []).filter((x) => groupOf(x.label) !== "battery").map((x) => x.celsius));
          setSamples((prev) => [
            ...prev,
            { t: s.elapsedSecs, cpu: live?.cpuTotal ?? 0, temp: hottest, freq: live?.cpuMhz ?? 0, power: powerRef.current, gpu: gpuRef.current, rate: s.opsPerSec },
          ]);
          if (hottest >= settings.stressLimit) api.stressStop("overheat").catch(() => {});
        } else if (wasRunning.current) {
          wasRunning.current = false;
          setResult(summarize(samplesRef.current, s));
        }
      } catch {}
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [settings.stressLimit]);

  const start = async () => {
    if (!(await ask(t("st.confirm"), { title: "Fairstack Coreview", kind: "warning" }))) return;
    setSamples([]);
    setResult(null);
    try {
      await api.stressStart(kind, threads, seconds);
      wasRunning.current = true;
    } catch {}
  };

  const durations = [30, 60, 300, 600, 1800];
  const label = (s: number) => (s < 60 ? `${s} ${t("set.seconds")}` : t("st.minutes", { n: s / 60 }));
  const current = samples[samples.length - 1];
  const usesCpu = kind !== "disk" && kind !== "gpu";
  const usesGpu = kind === "gpu" || kind === "full";
  const mops = (v: number) => `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(v / 1e6)} M/s`;
  const reasonText = result ? (result.reason === "error" ? t("st.gpuFail") : result.reason === "overheat" ? t("st.overheat") : result.reason === "stopped" ? t("st.stopped") : t("st.finished")) : "";
  const gflops = (v: number) => `${new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(v / 1e9)} GFLOPS`;
  const rate = (v: number) => (kind === "gpu" ? gflops(v) : mops(v));

  return (
    <>
      <PageHead
        title={t("nav.stress")}
        right={
          running ? (
            <button className="btn danger-btn" onClick={() => api.stressStop()}>
              <Icon name="pause" />
              {t("st.stop")}
            </button>
          ) : (
            <button className="btn primary" onClick={start}>
              <Icon name="play" />
              {t("st.start")}
            </button>
          )
        }
      />

      <Card>
        <Field label={t("st.type")}>
          <Segmented
            value={kind}
            onChange={(k) => !running && setKind(k)}
            options={[
              { value: "cpu", label: t("st.cpu") },
              { value: "memory", label: t("st.memory") },
              { value: "disk", label: t("st.disk") },
              { value: "gpu", label: t("st.gpu") },
              { value: "all", label: t("st.all") },
              { value: "full", label: t("st.full") },
            ]}
          />
        </Field>
        {usesCpu && (
          <Field label={t("st.threads")}>
            <Slider value={Math.min(threads, cores)} min={1} max={cores} onChange={(v) => !running && setThreads(v)} />
          </Field>
        )}
        <Field label={t("st.duration")}>
          <div className="langs">
            {durations.map((d) => (
              <button key={d} className={`chip ${seconds === d ? "active" : ""}`} onClick={() => !running && setSeconds(d)}>
                {label(d)}
              </button>
            ))}
          </div>
        </Field>
        <Field label={t("st.limit")}>
          <Slider value={settings.stressLimit} min={60} max={110} suffix="°C" onChange={(stressLimit) => set({ stressLimit })} />
        </Field>
        {kind === "disk" && <p className="muted small">{t("st.diskNote")}</p>}
        {usesGpu && <p className="muted small">{t("st.gpuNote")}</p>}
        {status?.error && <div className="note error">{t("st.gpuFail")}: {status.error}</div>}
        {usesGpu && status?.device && (
          <p className="muted small">
            {t("st.device")}: {status.device}
          </p>
        )}
        {running && status && (
          <div className="progress">
            <Bar value={(status.elapsedSecs / status.durationSecs) * 100} />
            <small>
              {t("st.running")} {duration(Math.round(status.elapsedSecs), locale) || "0"} / {duration(status.durationSecs, locale)}
            </small>
          </div>
        )}
      </Card>

      {(running || samples.length > 0) && (
        <>
          <div className="rings stats">
            <Card className="stat">
              <span>{t("mon.cpu")}</span>
              <strong>{pct(current?.cpu ?? 0, locale)}</strong>
            </Card>
            <Card className="stat">
              <span>{t("mon.temps")}</span>
              <strong>{current?.temp ? `${Math.round(current.temp)}°C` : "—"}</strong>
            </Card>
            <Card className="stat">
              <span>{t("mon.freq")}</span>
              <strong>{mhz(current?.freq ?? 0, locale)}</strong>
            </Card>
            <Card className="stat">
              <span>{kind === "disk" ? t("st.write") : kind === "full" ? t("mon.gpu") : t("st.score")}</span>
              <strong>
                {kind === "disk"
                  ? `${Math.round(status?.diskWriteMbs ?? 0)} MB/s`
                  : kind === "full"
                    ? pct(current?.gpu ?? 0, locale)
                    : kind === "gpu"
                      ? gflops(current?.rate ?? 0)
                      : mops(current?.rate ?? 0)}
              </strong>
            </Card>
          </div>
          <div className="grid">
            <Card title={t("mon.cpu")}>
              <Chart data={samples} max={100} series={[{ key: "cpu", color: "var(--accent)", name: t("mon.cpu") }]} format={(v) => pct(v, locale)} height={130} />
            </Card>
            <Card title={t("mon.temps")}>
              <Chart data={samples} series={[{ key: "temp", color: "var(--danger)", name: t("mon.temps") }]} format={(v) => `${v.toFixed(1)}°C`} height={130} />
            </Card>
            <Card title={t("mon.freq")}>
              <Chart data={samples} series={[{ key: "freq", color: "var(--c3)", name: t("mon.freq") }]} format={(v) => mhz(v, locale)} height={130} />
            </Card>
            {usesGpu && (
              <Card title={t("mon.gpu")}>
                <Chart data={samples} max={100} series={[{ key: "gpu", color: "var(--c2)", name: t("mon.gpu") }]} format={(v) => pct(v, locale)} height={130} />
              </Card>
            )}
            <Card title={t("pow.system")}>
              <Chart data={samples} series={[{ key: "power", color: "var(--c4)", name: t("pow.system") }]} format={(v) => `${v.toFixed(1)} W`} height={130} />
            </Card>
          </div>
        </>
      )}

      {result && !running && (
        <Card title={t("st.result")} right={<Badge tone={result.reason === "overheat" || result.throttled ? "warn" : "ok"}>{reasonText}</Badge>} className="result">
          {result.kind !== "disk" && (
            <div className={`verdict ${result.throttled ? "warn" : "ok"}`}>{result.throttled ? t("st.throttle") : t("st.stable")}</div>
          )}
          <div className="cols">
            <div className="row"><span>{t("st.duration")}</span><b>{duration(Math.round(result.seconds), locale)}</b></div>
            <div className="row"><span>{t("st.avgTemp")}</span><b>{result.avgTemp ? `${result.avgTemp.toFixed(1)}°C` : "—"}</b></div>
            <div className="row"><span>{t("st.maxTemp")}</span><b>{result.maxTemp ? `${result.maxTemp.toFixed(1)}°C` : "—"}</b></div>
            {result.kind !== "disk" && (
              <div className="row"><span>{t("st.freq")}</span><b>{`${result.freqChange > 0 ? "+" : ""}${result.freqChange.toFixed(1)}%`}</b></div>
            )}
            {result.avgPower !== null && <div className="row"><span>{t("st.avgPower")}</span><b>{result.avgPower.toFixed(1)} W</b></div>}
            {result.peakPower !== null && <div className="row"><span>{t("st.peakPower")}</span><b>{result.peakPower.toFixed(1)} W</b></div>}
            {result.kind === "disk" ? (
              <>
                <div className="row"><span>{t("st.write")}</span><b>{Math.round(result.write)} MB/s</b></div>
                <div className="row"><span>{t("st.read")}</span><b>{Math.round(result.read)} MB/s</b></div>
              </>
            ) : result.kind === "full" ? null : (
              <div className="row"><span>{t("st.score")}</span><b>{rate(result.score)}</b></div>
            )}
            {result.device && <div className="row"><span>{t("st.device")}</span><b>{result.device}</b></div>}
          </div>
        </Card>
      )}
    </>
  );
}
