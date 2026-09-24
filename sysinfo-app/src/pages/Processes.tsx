import { useEffect, useMemo, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { api, type ProcInfo } from "../api";
import { bytes, duration, pct } from "../format";
import { useSettings } from "../settings";
import { Card, Icon, PageHead } from "../ui";

type SortKey = "cpu" | "memory" | "name" | "pid" | "runTime";

export default function Processes() {
  const { t, locale } = useSettings();
  const [list, setList] = useState<ProcInfo[]>([]);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<{ key: SortKey; dir: 1 | -1 }>({ key: "cpu", dir: -1 });

  useEffect(() => {
    let alive = true;
    const load = () => api.processes().then((p) => alive && setList(p)).catch(() => {});
    load();
    const id = setInterval(load, 2000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = q ? list.filter((p) => p.name.toLowerCase().includes(q) || String(p.pid).includes(q)) : list;
    return [...filtered].sort((a, b) => {
      const x = a[sort.key];
      const y = b[sort.key];
      return (typeof x === "string" ? x.localeCompare(y as string) : (x as number) - (y as number)) * sort.dir;
    });
  }, [list, query, sort]);

  const head = (key: SortKey, label: string) => (
    <th className={`sortable ${sort.key === key ? "sorted" : ""}`} onClick={() => setSort((s) => ({ key, dir: s.key === key ? (-s.dir as 1 | -1) : -1 }))}>
      {label}
      {sort.key === key && (sort.dir === -1 ? " ↓" : " ↑")}
    </th>
  );

  const end = async (p: ProcInfo) => {
    if (await ask(t("proc.confirm", { name: p.name }), { title: "Fairstack Coreview", kind: "warning" })) {
      await api.kill(p.pid);
    }
  };

  return (
    <>
      <PageHead title={t("nav.processes")} sub={t("proc.count", { n: list.length })} />
      <Card>
        <div className="search">
          <Icon name="search" />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("common.search")} />
        </div>
        <div className="table-wrap tall">
          <table>
            <thead>
              <tr>
                {head("name", t("common.name"))}
                {head("pid", t("proc.pid"))}
                {head("cpu", t("proc.cpu"))}
                {head("memory", t("proc.mem"))}
                {head("runTime", t("proc.time"))}
                <th />
              </tr>
            </thead>
            <tbody>
              {rows.map((p) => (
                <tr key={p.pid}>
                  <td title={p.exe}>{p.name}</td>
                  <td className="mono">{p.pid}</td>
                  <td>{pct(p.cpu, locale)}</td>
                  <td>{bytes(p.memory, locale)}</td>
                  <td>{duration(p.runTime, locale)}</td>
                  <td>
                    <button className="icon-btn danger" title={t("proc.end")} onClick={() => end(p)}>
                      <Icon name="x" />
                    </button>
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
