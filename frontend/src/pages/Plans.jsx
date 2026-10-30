import { useState } from "react";
import {
  HardDrive,
  Check,
  Globe2,
  Clock3,
  Play,
  Pencil,
  Trash2,
  Plus,
  ArrowRight,
  Search,
  Folder,
  ShieldCheck,
} from "lucide-react";
import { useApp } from "../App";
import {
  Badge,
  Button,
  IconButton,
  Empty,
  Tabs,
  ErrorNotice,
} from "../components/ui";
import { api } from "../lib/api";
export function PlansPage() {
  const {
    plans,
    runs,
    metrics,
    remotes,
    status,
    t,
    say,
    openPlan,
    openRun,
    confirm,
    action,
    navigate,
  } = useApp();
  const [filter, setFilter] = useState(0);
  const [query, setQuery] = useState("");
  const [pending, setPending] = useState(null);
  const recent = runs
    .slice(0, 20)
    .filter((run) => ["success", "failed"].includes(run.status));
  const rate = recent.length
    ? `${Math.round((recent.filter((run) => run.status === "success").length / recent.length) * 100)}%`
    : "—";
  const ready = status.rclone_ready && !status.rclone_quarantined;
  const selected = plans.filter(
    (plan) =>
      (filter === 0 || plan.enabled === (filter === 1)) &&
      plan.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  return (
    <>
      {!ready && (
        <div className="setup-notice">
          <ShieldCheck size={24} />
          <div>
            <strong>
              {t(
                status.rclone_quarantined
                  ? "rcloneQuarantined"
                  : "rcloneWaiting",
              )}
            </strong>
            <p>
              {t(
                status.rclone_quarantined
                  ? "rcloneQuarantinedHint"
                  : "rcloneWaitingHint",
              )}
            </p>
          </div>
          {!status.rclone_quarantined && (
            <Button onClick={() => navigate("accounts")}>
              {t("configureStorage")}
              <ArrowRight size={16} />
            </Button>
          )}
        </div>
      )}
      <section className="metrics" aria-label={t("metrics")}>
        {[
          [
            HardDrive,
            t("activePlans"),
            plans.filter((p) => p.enabled).length,
            t("automatic"),
            "blue",
          ],
          [
            Check,
            t("last24"),
            metrics?.successful_runs_24h ?? "—",
            t("successRuns"),
            "green",
          ],
          [ShieldCheck, t("successRate"), rate, t("recentRuns"), "yellow"],
          [Globe2, t("destinations"), remotes.length, "rclone", "red"],
        ].map(([Icon, label, value, hint, color]) => (
          <article className={`metric ${color}`} key={label}>
            <div className="metric-top">
              <span>{label}</span>
              <Icon size={18} />
            </div>
            <strong>{value}</strong>
            <small>{hint}</small>
          </article>
        ))}
      </section>
      <section className="content-section">
        <div className="section-toolbar">
          <Tabs
            tabs={[
              say("全部方案", "All plans"),
              t("enabledBadge"),
              t("disabledBadge"),
            ]}
            value={filter}
            onChange={setFilter}
            label={t("backupPlans")}
            panels={[null, null, null]}
          />
          <label className="search-field">
            <Search size={16} />
            <input
              aria-label={say("搜索方案", "Search plans")}
              placeholder={say("搜索方案…", "Search plans…")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
        </div>
        {!plans.length ? (
          <Empty title={t("noPlans")} hint={t("noPlansHint")}>
            <Button variant="primary" onClick={() => openPlan()}>
              <Plus size={16} />
              {t("createPlan")}
            </Button>
          </Empty>
        ) : !selected.length ? (
          <Empty title={say("没有匹配的方案", "No matching plans")} />
        ) : (
          <div className="plan-grid">
            {selected.map((plan) => (
              <article className="plan-card" key={plan.id}>
                <div className="plan-top">
                  <span className="object-icon">
                    <Folder size={22} />
                  </span>
                  <Badge status={plan.enabled ? "success" : "muted"}>
                    {t(plan.enabled ? "enabledBadge" : "disabledBadge")}
                  </Badge>
                </div>
                <h2>{plan.name}</h2>
                <div className="plan-schedule">
                  <Clock3 size={14} />
                  <code>{plan.schedule}</code>
                  <span>{plan.timezone}</span>
                </div>
                <div className="plan-paths">
                  <div>
                    <span>{t("source")}</span>
                    <strong title={plan.sources.map((s) => s.path).join("\n")}>
                      {plan.sources.map((s) => s.name).join(", ")}
                    </strong>
                  </div>
                  <div>
                    <span>{t("target")}</span>
                    <strong
                      title={plan.remotes
                        .map((r) => `${r.name}:${r.directory}`)
                        .join("\n")}
                    >
                      {plan.remotes.map((r) => r.name).join(", ")}
                    </strong>
                  </div>
                </div>
                <div className="plan-footer">
                  <Button
                    variant="primary"
                    disabled={!ready}
                    busy={pending === plan.id}
                    onClick={async () => {
                      setPending(plan.id);
                      try {
                        const result = await action(
                          () =>
                            api(`/api/plans/${plan.id}/run`, {
                              method: "POST",
                            }),
                          t("runQueued"),
                        );
                        openRun(result.run_id);
                      } catch {
                      } finally {
                        setPending(null);
                      }
                    }}
                  >
                    <Play size={15} />
                    {t("runNow")}
                  </Button>
                  <div>
                    <IconButton
                      icon={Pencil}
                      label={`${t("edit")} ${plan.name}`}
                      onClick={() => openPlan(plan)}
                    />
                    <IconButton
                      icon={Trash2}
                      label={`${t("delete")} ${plan.name}`}
                      onClick={() =>
                        confirm({
                          title: `${t("delete")} · ${plan.name}`,
                          description: t("deleteConfirm"),
                          onConfirm: () =>
                            action(
                              () =>
                                api(`/api/plans/${plan.id}`, {
                                  method: "DELETE",
                                }),
                              t("deleteSuccess"),
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
      </section>
    </>
  );
}
