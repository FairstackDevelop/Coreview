export function bytes(n: number, locale = "en-US"): string {
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = v >= 100 || i === 0 ? 0 : v >= 10 ? 1 : 2;
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(v)} ${units[i]}`;
}

export const rate = (n: number, locale?: string) => `${bytes(n, locale)}/s`;

export const pct = (n: number, locale = "en-US") =>
  `${new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(n)}%`;

export function mhz(n: number, locale = "en-US"): string {
  if (!n) return "—";
  return n >= 1000
    ? `${new Intl.NumberFormat(locale, { maximumFractionDigits: 2 }).format(n / 1000)} GHz`
    : `${n} MHz`;
}

export function duration(secs: number, locale = "en-US"): string {
  const parts: [number, string][] = [
    [Math.floor(secs / 86400), "day"],
    [Math.floor((secs % 86400) / 3600), "hour"],
    [Math.floor((secs % 3600) / 60), "minute"],
  ];
  if (secs < 60) parts.push([Math.floor(secs), "second"]);
  return parts
    .filter(([v]) => v > 0)
    .slice(0, 2)
    .map(([v, unit]) => new Intl.NumberFormat(locale, { style: "unit", unit, unitDisplay: "narrow" }).format(v))
    .join(" ") || "0";
}

export function flatten(value: unknown, prefix = "", out: Record<string, string> = {}): Record<string, string> {
  if (Array.isArray(value)) {
    value.forEach((v, i) => flatten(v, `${prefix}[${i}]`, out));
  } else if (value && typeof value === "object") {
    for (const [k, v] of Object.entries(value)) flatten(v, prefix ? `${prefix}.${k}` : k, out);
  } else if (prefix) {
    out[prefix] = value === null || value === undefined ? "" : String(value);
  }
  return out;
}

export function download(name: string, content: string, mime: string) {
  const url = URL.createObjectURL(new Blob([content], { type: mime }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
