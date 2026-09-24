import { useEffect, useMemo, useState } from "react";
import { api, type DetailNode } from "../api";
import type { Key } from "../i18n";
import { saveAs } from "../report";
import { useSettings } from "../settings";
import { Empty, Icon, PageHead } from "../ui";

const pretty = (key: string) => {
  if (key.includes(".")) return key;
  const s = key
    .replace(/^sp[a-z]*?_/i, "")
    .replace(/^_+/, "")
    .replace(/_+/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/\s+/g, " ")
    .trim();
  return s.charAt(0).toUpperCase() + s.slice(1);
};

function prune(node: DetailNode, q: string): DetailNode | null {
  if (node.name.toLowerCase().includes(q)) return node;
  const props = node.props.filter(([k, v]) => `${k} ${v}`.toLowerCase().includes(q));
  const children = node.children.map((c) => prune(c, q)).filter((c): c is DetailNode => c !== null);
  return props.length || children.length ? { ...node, props, children } : null;
}

function Tree({ node, depth, force }: { node: DetailNode; depth: number; force: boolean }) {
  const { t, locale } = useSettings();
  const [open, setOpen] = useState(depth === 0);
  const expanded = open || force;

  const value = (v: string) => {
    if (v === "true") return t("common.yes");
    if (v === "false") return t("common.no");
    const m = /^\/Date\((-?\d+)\)\/$/.exec(v);
    return m ? new Date(Number(m[1])).toLocaleString(locale) : v;
  };

  return (
    <details className={`node d${Math.min(depth, 3)}`} open={expanded} onToggle={(e) => setOpen(e.currentTarget.open)}>
      <summary>
        <span>{node.name}</span>
        <small>{node.props.length + node.children.length}</small>
      </summary>
      {expanded && (
        <div className="node-body">
          {node.props.map(([k, v], i) => (
            <div key={i} className="prop">
              <span>{pretty(k)}</span>
              <b>{value(v)}</b>
            </div>
          ))}
          {node.children.map((c, i) => (
            <Tree key={i} node={c} depth={depth + 1} force={force} />
          ))}
        </div>
      )}
    </details>
  );
}

export default function Details() {
  const { t } = useSettings();
  const [cats, setCats] = useState<string[]>([]);
  const [cat, setCat] = useState("computer");
  const [cache, setCache] = useState<Record<string, DetailNode[]>>({});
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");

  useEffect(() => {
    api.detailCategories().then(setCats).catch(() => {});
  }, []);

  useEffect(() => {
    if (cache[cat]) return;
    let alive = true;
    setLoading(true);
    api
      .detailData(cat)
      .then((d) => alive && setCache((c) => ({ ...c, [cat]: d })))
      .catch(() => alive && setCache((c) => ({ ...c, [cat]: [] })))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [cat, cache]);

  const q = query.trim().toLowerCase();
  const nodes = useMemo(() => {
    const data = cache[cat] ?? [];
    return q ? data.map((n) => prune(n, q)).filter((n): n is DetailNode => n !== null) : data;
  }, [cache, cat, q]);

  const total = useMemo(() => {
    const count = (n: DetailNode): number => n.props.length + n.children.reduce((a, c) => a + count(c), 0);
    return (cache[cat] ?? []).reduce((a, n) => a + count(n), 0);
  }, [cache, cat]);

  const exportJson = () => {
    const stamp = new Date().toISOString().slice(0, 10);
    saveAs(`coreview-${cat}-${stamp}.json`, "json", JSON.stringify(cache[cat] ?? [], null, 2)).catch(() => {});
  };

  return (
    <>
      <PageHead
        title={t("nav.details")}
        sub={cache[cat] ? t("det.props", { n: total }) : undefined}
        right={
          <button className="btn" onClick={exportJson} disabled={!cache[cat]?.length}>
            <Icon name="download" />
            {t("exp.json")}
          </button>
        }
      />
      <div className="details">
        <nav className="cats">
          {cats.map((c) => (
            <button key={c} className={c === cat ? "active" : ""} onClick={() => setCat(c)}>
              {t(`cat.${c}` as Key)}
            </button>
          ))}
        </nav>
        <div className="detail-pane">
          <div className="search">
            <Icon name="search" />
            <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("det.search")} />
          </div>
          {loading && !cache[cat] ? (
            <Empty>{t("common.loading")}</Empty>
          ) : nodes.length === 0 ? (
            <Empty>{t("det.empty")}</Empty>
          ) : (
            <div className="tree">
              {nodes.map((n, i) => (
                <Tree key={`${cat}-${i}-${q}`} node={n} depth={0} force={!!q} />
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  );
}
