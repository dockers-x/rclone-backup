import { useState } from "react";
import {
  Globe2,
  Plus,
  Pencil,
  Trash2,
  PlugZap,
  ArrowUpRight,
} from "lucide-react";
import { useApp } from "../App";
import { Button, IconButton, Empty, Badge } from "../components/ui";
import { api } from "../lib/api";
export function AccountsPage() {
  const { remotes, t, say, openRemote, confirm, action } = useApp();
  const [pending, setPending] = useState(null);
  return (
    <>
      {!remotes.length ? (
        <Empty icon={Globe2} title={t("noAccounts")} hint={t("noAccountsHint")}>
          <Button variant="primary" onClick={() => openRemote()}>
            <Plus size={16} />
            {t("addAccount")}
          </Button>
        </Empty>
      ) : (
        <div className="account-grid">
          {remotes.map((remote) => (
            <article className="account-card" key={remote.name}>
              <div className="account-head">
                <span className="object-icon blue">
                  <Globe2 size={23} />
                </span>
                <Badge status="muted">{remote.type}</Badge>
              </div>
              <h2>{remote.name}</h2>
              <p>
                {say(
                  "凭据由 rclone 加密保存",
                  "Credentials encrypted by rclone",
                )}
              </p>
              <div className="plan-footer">
                <Button
                  busy={pending === remote.name}
                  onClick={async () => {
                    setPending(remote.name);
                    try {
                      await action(
                        () =>
                          api(
                            `/api/rclone/remotes/${encodeURIComponent(remote.name)}/test`,
                            { method: "POST" },
                          ),
                        t("testSuccess"),
                      );
                    } catch {
                    } finally {
                      setPending(null);
                    }
                  }}
                >
                  <PlugZap size={16} />
                  {t("test")}
                </Button>
                <div>
                  <IconButton
                    icon={Pencil}
                    label={`${t("edit")} ${remote.name}`}
                    onClick={() => openRemote(remote)}
                  />
                  <IconButton
                    icon={Trash2}
                    label={`${t("delete")} ${remote.name}`}
                    onClick={() =>
                      confirm({
                        title: `${t("delete")} · ${remote.name}`,
                        description: t("deleteRemoteConfirm"),
                        onConfirm: () =>
                          action(
                            () =>
                              api(
                                `/api/rclone/remotes/${encodeURIComponent(remote.name)}`,
                                { method: "DELETE" },
                              ),
                            t("remoteDeleted"),
                          ),
                      })
                    }
                  />
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
      <div className="info-foot">
        <Globe2 size={16} />
        <span>
          {say(
            "支持 S3、WebDAV、SFTP 及所有 rclone 存储提供商。",
            "Connect S3, WebDAV, SFTP, and any rclone storage provider.",
          )}
        </span>
      </div>
    </>
  );
}
