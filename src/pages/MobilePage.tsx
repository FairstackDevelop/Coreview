import { useCallback, useEffect, useState } from "react";
import QRCode from "qrcode";
import { api, type AgentStatus, type PairingInfo } from "../api";
import { useSettings } from "../settings";
import { Badge, Card, Empty, Field, Icon, PageHead, Toggle } from "../ui";

export default function MobilePage() {
  const { t, locale } = useSettings();
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [pairing, setPairing] = useState<PairingInfo | null>(null);
  const [qr, setQr] = useState<string>("");
  const [left, setLeft] = useState(0);
  const [note, setNote] = useState<string | null>(null);
  const [port, setPort] = useState<number | null>(null);

  const refresh = useCallback(() => {
    api.agentStatus().then((s) => {
      setStatus(s);
      setPort((p) => p ?? s.port);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 3000);
    return () => clearInterval(id);
  }, [refresh]);

  useEffect(() => {
    if (!pairing) return;
    QRCode.toString(pairing.link, { type: "svg", margin: 1, color: { dark: "#0b0c12", light: "#ffffff" } }).then(setQr).catch(() => setQr(""));
    setLeft(pairing.expiresIn);
    const id = setInterval(() => setLeft((v) => (v > 0 ? v - 1 : 0)), 1000);
    return () => clearInterval(id);
  }, [pairing]);

  useEffect(() => {
    if (pairing && left === 0) {
      setPairing(null);
      setQr("");
    }
  }, [left, pairing]);

  const newPairing = async () => {
    try {
      setPairing(await api.agentPairing());
    } catch (e) {
      setNote(String(e));
    }
  };

  const copy = async () => {
    if (!pairing) return;
    try {
      await navigator.clipboard.writeText(pairing.link);
      setNote(t("mb.copied"));
    } catch {}
  };

  if (!status) return <div className="empty">{t("common.loading")}</div>;
  const seen = (ms: number) => (ms ? new Date(ms).toLocaleString(locale) : "—");

  return (
    <>
      <PageHead
        title={t("nav.mobile")}
        right={<Badge tone={status.running ? "ok" : "muted"}>{status.running ? t("mb.running") : t("mb.stopped")}</Badge>}
      />
      {note && <div className="note">{note}</div>}
      <div className="grid">
        <Card>
          <Field label={t("mb.enable")}>
            <Toggle checked={status.enabled} onChange={(v) => api.agentEnable(v).then(setStatus)} />
          </Field>
          <Field label={t("mb.control")}>
            <Toggle checked={status.allowControl} onChange={(v) => api.agentOptions(status.port, v).then(setStatus)} />
          </Field>
          <Field label={t("mb.port")}>
            <input
              className="port-input"
              type="number"
              min={1024}
              max={65535}
              value={port ?? status.port}
              onChange={(e) => setPort(Number(e.target.value))}
              onBlur={() => port && port !== status.port && api.agentOptions(port, status.allowControl).then(setStatus)}
            />
          </Field>
          <Field label={t("mb.address")}>
            <span className="mono">{status.addresses.length ? status.addresses.join(", ") : "—"}</span>
          </Field>
          <p className="muted small">{t("mb.hint")}</p>
          <p className="muted small">{t("mb.firewall")}</p>
        </Card>

        <Card title={t("mb.pair")}>
          {!status.running ? (
            <Empty>{t("mb.stopped")}</Empty>
          ) : pairing ? (
            <div className="pairing">
              <div className="qr" dangerouslySetInnerHTML={{ __html: qr }} />
              <p className="muted small">{t("mb.scan")}</p>
              <div className="code">
                <span>{t("mb.code")}</span>
                <b>{pairing.code.replace(/(\d{4})(\d{4})/, "$1 $2")}</b>
                <small>{Math.floor(left / 60)}:{String(left % 60).padStart(2, "0")}</small>
              </div>
              <button className="btn" onClick={copy}>
                <Icon name="download" />
                {t("mb.copy")}
              </button>
            </div>
          ) : (
            <button className="btn primary" onClick={newPairing}>
              <Icon name="plus" />
              {t("mb.pair")}
            </button>
          )}
        </Card>

        <Card title={t("mb.devices")} className="span2">
          {status.devices.length === 0 ? (
            <Empty>{t("mb.none")}</Empty>
          ) : (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>{t("common.name")}</th>
                    <th>{t("mb.seen")}</th>
                    <th>{t("mb.allow")}</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {status.devices.map((d) => (
                    <tr key={d.id}>
                      <td>{d.name}</td>
                      <td>{seen(d.lastSeen)}</td>
                      <td>
                        <Toggle checked={d.control} onChange={(v) => api.agentDeviceControl(d.id, v).then(setStatus)} />
                      </td>
                      <td>
                        <button className="icon-btn danger" title={t("mb.remove")} onClick={() => api.agentRevoke(d.id).then(setStatus)}>
                          <Icon name="trash" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>

        <Card title={t("mb.log")} className="span2">
          {status.events.length === 0 ? (
            <Empty>—</Empty>
          ) : (
            <div className="diffs">
              {status.events.slice(0, 20).map((e, i) => (
                <div key={i} className="diff">
                  <code>{new Date(e.t).toLocaleString(locale)}</code>
                  <span>{e.text}</span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </>
  );
}
