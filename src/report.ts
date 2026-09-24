import { save } from "@tauri-apps/plugin-dialog";
import { api, type HardwareInfo } from "./api";
import { flatten } from "./format";
import type { Translate } from "./i18n";

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

export function buildHtml(hw: HardwareInfo, t: Translate, locale: string, accent: string): string {
  const groups: Record<string, [string, string][]> = {};
  for (const [path, value] of Object.entries(flatten(hw))) {
    if (!value) continue;
    const [head, ...rest] = path.split(/\.(.+)/);
    (groups[head.replace(/\[\d+\]/, "")] ??= []).push([rest[0] ?? head, value]);
  }
  const sections = Object.entries(groups)
    .map(
      ([name, rows]) =>
        `<h2>${esc(name)}</h2><table>${rows.map(([k, v]) => `<tr><th>${esc(k)}</th><td>${esc(v)}</td></tr>`).join("")}</table>`,
    )
    .join("");
  return `<!doctype html><html lang="${locale}"><meta charset="utf-8"><title>${esc(t("exp.title"))}</title>
<style>body{font:14px system-ui;max-width:860px;margin:40px auto;padding:0 20px;color:#16181d}h1{margin:0}h2{margin:28px 0 8px;color:${accent};text-transform:capitalize}
p{color:#6b7280}table{width:100%;border-collapse:collapse}th,td{text-align:left;padding:6px 10px;border-bottom:1px solid #e5e7eb;vertical-align:top}th{width:38%;font-weight:500;color:#6b7280;font-family:ui-monospace,monospace;font-size:12px}</style>
<h1>${esc(t("exp.title"))}</h1><p>${esc(hw.os.hostname)} · ${esc(t("exp.generated"))} ${new Date().toLocaleString(locale)}</p>${sections}</html>`;
}

export async function saveAs(name: string, ext: string, content: string): Promise<string | null> {
  const path = await save({ defaultPath: name, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (!path) return null;
  await api.saveFile(path, content);
  return path;
}
