import { useEffect, useState } from "react";
import { api, type HardwareInfo, type SnapshotMeta } from "../api";
import { flatten } from "../format";
import { useSettings } from "../settings";
import { Card, Empty, Icon, PageHead } from "../ui";

const VOLATILE = /uptimeSecs|available|baseMhz/;

interface Diff {
  path: string;
  before?: string;
  now?: string;
}

function diff(before: HardwareInfo, now: HardwareInfo): Diff[] {
  const a = flatten(before);
  const b = flatten(now);
  const out: Diff[] = [];
  for (const path of new Set([...Object.keys(a), ...Object.keys(b)])) {
    if (VOLATILE.test(path) || a[path] === b[path]) continue;
    out.push({ path, before: a[path], now: b[path] });
  }
  return out.sort((x, y) => x.path.localeCompare(y.path));
}

export default function Snapshots({ hw }: { hw: HardwareInfo }) {
  const { t, locale } = useSettings();
  const [list, setList] = useState<SnapshotMeta[]>([]);
  const [name, setName] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [changes, setChanges] = useState<Diff[] | null>(null);
  const [aboutOpen] = useState(() => {
    try {
      return localStorage.getItem("coreview.snapAbout") !== "closed";
    } catch {
      return true;
    }
  });
  const rememberAbout = (open: boolean) => {
    try {
      localStorage.setItem("coreview.snapAbout", open ? "open" : "closed");
    } catch {}
  };

  const refresh = () => api.snapshots().then(setList).catch(() => {});
  useEffect(() => {
    refresh();
  }, []);

  const take = async () => {
    await api.saveSnapshot(name.trim() || new Date().toLocaleString(locale), hw);
    setName("");
    refresh();
  };

  const compare = async (id: string) => {
    setSelected(id);
    setChanges(diff(await api.loadSnapshot(id), hw));
  };

  const remove = async (id: string) => {
    await api.deleteSnapshot(id);
    if (selected === id) {
      setSelected(null);
      setChanges(null);
    }
    refresh();
  };

  return (
    <>
      <PageHead title={t("nav.snapshots")} />
      <details className="about" open={aboutOpen} onToggle={(e) => rememberAbout(e.currentTarget.open)}>
        <summary>
          <Icon name="info" />
          {t("snap.aboutTitle")}
        </summary>
        <p>{t("snap.aboutText")}</p>
        <div className="about-cols">
          <div>
            <h4>{t("snap.useTitle")}</h4>
            <ul>
              <li>{t("snap.use1")}</li>
              <li>{t("snap.use2")}</li>
              <li>{t("snap.use3")}</li>
            </ul>
          </div>
          <div>
            <h4>{t("snap.stepsTitle")}</h4>
            <ol>
              <li>{t("snap.step1")}</li>
              <li>{t("snap.step2")}</li>
              <li>{t("snap.step3")}</li>
            </ol>
          </div>
        </div>
        <small className="muted">{t("snap.privacy")}</small>
      </details>
      <div className="grid">
        <Card>
          <div className="inline-form">
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t("snap.placeholder")} onKeyDown={(e) => e.key === "Enter" && take()} />
            <button className="btn primary" onClick={take}>
              <Icon name="camera" />
              {t("snap.take")}
            </button>
          </div>
          {list.length === 0 ? (
            <Empty>{t("snap.empty")}</Empty>
          ) : (
            <ul className="list">
              {list.map((s) => (
                <li key={s.id} className={selected === s.id ? "active" : ""}>
                  <button className="list-main" onClick={() => compare(s.id)}>
                    <b>{s.name}</b>
                    <small>{new Date(s.created).toLocaleString(locale)}</small>
                  </button>
                  <button className="icon-btn danger" title={t("common.delete")} onClick={() => remove(s.id)}>
                    <Icon name="trash" />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card title={t("snap.compare")} right={changes && <small className="muted">{t("snap.changes", { n: changes.length })}</small>}>
          {!changes ? (
            <Empty>{t("snap.select")}</Empty>
          ) : changes.length === 0 ? (
            <Empty>{t("snap.identical")}</Empty>
          ) : (
            <div className="diffs">
              {changes.map((c) => (
                <div key={c.path} className="diff">
                  <code>{c.path}</code>
                  <span className="del">{c.before || "∅"}</span>
                  <span className="add">{c.now || "∅"}</span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </>
  );
}
