import type { SmcTemp } from "./api";
import type { Key, Translate } from "./i18n";

export type Group = "chip" | "gpu" | "memory" | "board" | "storage" | "battery" | "ambient" | "other";

export const groupOrder: Group[] = ["chip", "gpu", "memory", "board", "storage", "battery", "ambient", "other"];

export function groupOf(label: string): Group {
  const l = label.toLowerCase();
  if (/battery|gas gauge/.test(l)) return "battery";
  if (/nand|ssd|nvme|drive|composite/.test(l)) return "storage";
  if (/gpu|graphics/.test(l)) return "gpu";
  if (/tdev/.test(l)) return "board";
  if (/tdie|cpu|core|package|soc|die/.test(l)) return "chip";
  return "other";
}

export const smcGroup = (g: string): Group => (g === "cpu" ? "chip" : (groupOrder.includes(g as Group) ? (g as Group) : "other"));

export const tone = (c: number) => (c >= 90 ? "danger" : c >= 75 ? "warn" : undefined);

export function describe(label: string, t: Translate): string {
  const n = (re: RegExp) => re.exec(label)?.[1] ?? "";
  if (/tdie/i.test(label)) return t("sn.die", { n: n(/tdie\s*(\d+)/i) });
  if (/tdev/i.test(label)) return t("sn.dev", { n: n(/tdev\s*(\d+)/i) });
  if (/nand/i.test(label)) return t("sn.nand", { n: n(/ch\s*(\d+)/i) });
  if (/gas gauge|battery/i.test(label)) return t("tg.battery");
  return label;
}

export function describeSmc(s: SmcTemp, t: Translate): string {
  if (s.kind) return t(`sk.${s.kind}` as Key, { n: s.index ?? "" }).trim();
  if (s.name) return s.name;
  if (s.group === "battery") return `${t("tg.battery")} ${s.key.charAt(2)}`;
  return s.key;
}
