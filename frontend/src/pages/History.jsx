import { useEffect, useState } from "react";
import {
  History,
  ChevronRight,
  CheckCircle2,
  AlertCircle,
  Clock3,
  Ban,
  FileText,
} from "lucide-react";
import { useApp } from "../App";
import {
  Badge,
  Button,
  Empty,
  Modal,
  Loading,
  ErrorNotice,
  useNow,
} from "../components/ui";
import { api } from "../lib/api";
import { parseRunProgress, normalizeRunProgress } from "../lib/helpers";
export function HistoryPage() {
  const { runs, t, language, say, openRun } = useApp();
  return !runs.length ? (
    <Empty
      icon={History}
      title={t("noRuns")}
      hint={say(
        "运行一个方案后，记录会显示在这里。",
        "Run a backup plan to see its history here.",
      )}
    />
  ) : (
    <div className="history-table">
      <div className="history-labels">
        <span>{t("planName")}</span>
        <span>{say("开始时间", "Started")}</span>
        <span>{say("触发方式", "Trigger")}</span>
        <span>{say("状态", "Status")}</span>
        <span />
      </div>
      {runs.map((run) => (
        <button
          className="history-row"
          key={run.id}
          onClick={() => openRun(run.id)}
          aria-label={`${run.plan_name} · ${t(run.status)} · ${t("viewLog")}`}
        >
          <span className="run-name">
            <span className={`run-icon ${run.status}`}>
              {run.status === "success" ? (
                <CheckCircle2 size={17} />
              ) : run.status === "failed" ? (
                <AlertCircle size={17} />
              ) : (
                <Clock3 size={17} />
              )}
            </span>
            <strong>{run.plan_name}</strong>
          </span>
          <time dateTime={run.started_at}>
            {new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en", {
              dateStyle: "medium",
              timeStyle: "short",
            }).format(new Date(run.started_at))}
          </time>
          <span className="muted">
            {t(run.trigger === "schedule" ? "scheduleTrigger" : run.trigger)}
          </span>
          <Badge status={run.status}>{t(run.status)}</Badge>
          <ChevronRight size={16} />
        </button>
      ))}
    </div>
  );
}
export function RunModal({ id, onClose }) {
  const { t, say, notify, refresh, plans } = useApp();
  const { run, setRun, error, setError } = useRunDetails(id);
  const [busy, setBusy] = useState(false);
  const now = useNow();

  const progress = run
    ? normalizeRunProgress(run, parseRunProgress(run))
    : null;
  const elapsed = run
    ? Math.max(
        0,
        Math.floor(
          ((run.finished_at ? Date.parse(run.finished_at) : now) -
            Date.parse(run.started_at)) /
            1000,
        ),
      )
    : 0;
  return (
    <Modal
      title={run ? run.plan_name : t("runLog")}
      description={t("runProgress")}
      onClose={onClose}
      closeLabel={t("close")}
      wide
    >
      <div className="modal-body">
        <ErrorNotice>{error}</ErrorNotice>
        {!run ? (
          <Loading />
        ) : (
          <>
            <div className="run-summary">
              <Badge status={run.status}>{t(run.status)}</Badge>
              <span className="mono">
                {Math.floor(elapsed / 60)}m {elapsed % 60}s
              </span>
              <span>
                {t("attemptLabel")
                  .replace("{current}", run.attempt ?? 1)
                  .replace(
                    "{total}",
                    plans.find((plan) => plan.id === run.plan_id)?.retry
                      ?.max_attempts ||
                      run.attempt ||
                      1,
                  )}
              </span>
            </div>
            <p className="run-updated muted">
              {say("上次更新", "Last update")}:{" "}
              {new Date(
                run.finished_at || progress.updatedAt || run.started_at,
              ).toLocaleString()}
            </p>
            {run.error && <ErrorNotice>{run.error}</ErrorNotice>}
            <h3 className="subheading">
              {say("当前阶段", "Current phase")}: {t(progress.phase)}
            </h3>
            <div className="target-progress-head">
              <span>{t("targetProgress")}</span>
              <span>
                {
                  [...progress.targets.values()].filter(
                    (target) =>
                      !["pending", "checking", "uploading"].includes(
                        target.status,
                      ),
                  ).length
                }{" "}
                / {progress.targets.size}
              </span>
            </div>
            {progress.targets.size > 0 && (
              <progress
                className="target-progress-bar"
                aria-label={t("targetProgress")}
                max={progress.targets.size}
                value={
                  [...progress.targets.values()].filter(
                    (target) =>
                      !["pending", "checking", "uploading"].includes(
                        target.status,
                      ),
                  ).length
                }
              />
            )}
            {run.status === "running" && (
              <p className="field-help">{t("uploadTimeoutHint")}</p>
            )}
            <div className="target-list">
              {[...progress.targets].map(([key, target]) => (
                <div key={key}>
                  <GlobeIcon />
                  <span>
                    <strong>{target.name}</strong>
                    <code>{target.directory}</code>
                    {target.detail && (
                      <small className="target-detail">{target.detail}</small>
                    )}
                    {target.startedAt && (
                      <small className="target-timing">
                        {t("targetElapsed").replace(
                          "{time}",
                          `${Math.max(0, Math.floor(((target.at && !["checking", "uploading"].includes(target.status) ? Date.parse(target.at) : now) - Date.parse(target.startedAt)) / 1000))}s`,
                        )}
                      </small>
                    )}
                  </span>
                  <Badge status={target.status}>{t(target.status)}</Badge>
                </div>
              ))}
            </div>
            <details className="log-details" open={!progress.targets.size}>
              <summary>
                <FileText size={16} />
                {t("technicalLog")}
              </summary>
              <pre>
                {progress.log.join("\n") ||
                  say("暂无日志", "No log output yet")}
              </pre>
            </details>
            {["running", "pending", "retrying", "cancelling"].includes(
              run.status,
            ) && (
              <div className="modal-footer">
                <Button
                  variant="danger"
                  busy={busy}
                  disabled={run.status === "cancelling"}
                  onClick={async () => {
                    setBusy(true);
                    try {
                      await api(`/api/runs/${id}/cancel`, { method: "POST" });
                      notify(t("cancelRequested"));
                      setRun({ ...run, status: "cancelling" });
                      refresh(true);
                    } catch (e) {
                      setError(e.message);
                    } finally {
                      setBusy(false);
                    }
                  }}
                >
                  <Ban size={16} />
                  {t("cancelRun")}
                </Button>
              </div>
            )}
          </>
        )}
      </div>
    </Modal>
  );
}
function GlobeIcon() {
  return (
    <span className="object-icon small">
      <History size={17} />
    </span>
  );
}

export function useRunDetails(id) {
  const [run, setRun] = useState(null);
  const [error, setError] = useState("");
  useEffect(() => {
    setRun(null);
    setError("");
    let alive = true;
    let timer;
    const load = async () => {
      try {
        const result = await api(`/api/runs/${id}`);
        if (alive) {
          setRun(result);
          setError("");
        }
      } catch (e) {
        if (alive) setError(e.message);
      } finally {
        if (alive) timer = setTimeout(load, 2000);
      }
    };
    load();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [id]);
  return { run, setRun, error, setError };
}
