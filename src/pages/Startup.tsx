import { useCallback, useEffect, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { api, type StartupItem } from "../api";
import type { Key } from "../i18n";
import { useSettings } from "../settings";
import { Badge, Card, Empty, Icon, PageHead, Toggle } from "../ui";

export default function Startup() {
  const { t } = useSettings();
  const [items, setItems] = useState<StartupItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.startup().then(setItems).catch(() => setItems([]));
  }, []);
  useEffect(load, [load]);

  const guard = async (job: Promise<unknown>) => {
    try {
      await job;
      setError(null);
    } catch (e) {
      setError(String(e));
    }
    load();
  };

  const toggle = (item: StartupItem, enabled: boolean) => {
    setItems((list) => list?.map((i) => (i.id === item.id ? { ...i, enabled } : i)) ?? null);
    guard(api.startupToggle(item.id, enabled));
  };

  const remove = async (item: StartupItem) => {
    if (await ask(t("start.confirmRemove", { name: item.name }), { title: "Fairstack Coreview", kind: "warning" })) {
      guard(api.startupRemove(item.id));
    }
  };

  const add = async () => {
    const path = await open({ multiple: false, directory: false });
    if (typeof path === "string") guard(api.startupAdd(path));
  };

  return (
    <>
      <PageHead
        title={t("start.title")}
        sub={items ? String(items.length) : undefined}
        right={
          <button className="btn primary" onClick={add}>
            <Icon name="plus" />
            {t("start.add")}
          </button>
        }
      />
      {error && <div className="note error">{error}</div>}
      <Card>
        {items && items.length === 0 ? (
          <Empty>{t("start.empty")}</Empty>
        ) : (
          <div className="table-wrap tall">
            <table>
              <thead>
                <tr>
                  <th>{t("start.enabled")}</th>
                  <th>{t("common.name")}</th>
                  <th>{t("start.location")}</th>
                  <th>{t("start.command")}</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {items?.map((s) => (
                  <tr key={s.id} className={s.enabled ? "" : "dim"}>
                    <td>{s.canToggle ? <Toggle checked={s.enabled} onChange={(v) => toggle(s, v)} /> : <Badge tone="muted">{t("start.readonly")}</Badge>}</td>
                    <td>{s.name}</td>
                    <td>
                      <Badge tone="muted">{t(`loc.${s.location}` as Key)}</Badge>
                    </td>
                    <td className="mono clip" title={s.command}>
                      {s.command}
                    </td>
                    <td>
                      {s.canRemove && (
                        <button className="icon-btn danger" title={t("start.remove")} onClick={() => remove(s)}>
                          <Icon name="trash" />
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </>
  );
}
