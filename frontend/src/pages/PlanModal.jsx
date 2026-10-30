import { useState } from "react";
import { Plus, Trash2, Eye, EyeOff, Folder, Globe2 } from "lucide-react";
import { useApp } from "../App";
import {
  Button,
  Field,
  IconButton,
  Modal,
  Toggle,
  ErrorNotice,
} from "../components/ui";
import { api } from "../lib/api";
import { parseSimpleSchedule, splitArgs, joinArgs } from "../lib/helpers";
export function scheduleCron(simple) {
  const [hour, minute] = (simple.time || "02:00").split(":").map(Number);
  const interval = Number(simple.interval || 1);
  if (simple.kind === "daily") return `0 ${minute} ${hour} * * *`;
  if (simple.kind === "weekly")
    return `0 ${minute} ${hour} * * ${simple.weekday || "MON"}`;
  if (simple.kind === "monthly")
    return `0 ${minute} ${hour} ${simple.monthday || 1} * *`;
  if (simple.kind === "every_hours") return `0 0 0/${interval} * * *`;
  if (simple.kind === "every_minutes") return `0 0/${interval} * * * *`;
  return `0/${interval} * * * * *`;
}
export function PlanModal({ plan, onClose }) {
  const { t, say, remotes, status, notify, refresh } = useApp();
  const [draft, setDraft] = useState(() =>
    structuredClone(
      plan || {
        name: "",
        enabled: true,
        schedule: "0 0 2 * * *",
        timezone: "UTC",
        sources: [{ name: "data", path: "/data" }],
        remotes: [
          { name: remotes[0]?.name || "", directory: "/RcloneBackup/" },
        ],
        archive: {
          kind: "7z",
          password: "",
          password_hint: "",
          suffix: "%Y%m%d-%H%M%S",
        },
        retention: { keep_days: 0, keep_count: 0 },
        retry: {
          max_attempts: 3,
          initial_delay_seconds: 10,
          max_delay_seconds: 300,
          backoff: "exponential",
        },
        rclone_flags: [],
        remote_check_concurrency: 4,
        upload_concurrency: 1,
        notifications: { ping: {}, mail: {}, serverchan: {} },
      },
    ),
  );
  const parsed = parseSimpleSchedule(draft.schedule);
  const [simple, setSimple] = useState(
    parsed || {
      kind: "daily",
      time: "02:00",
      interval: 6,
      weekday: "MON",
      monthday: 1,
    },
  );
  const [mode, setMode] = useState(parsed ? "simple" : "cron");
  const [flags, setFlags] = useState(joinArgs(draft.rclone_flags));
  const [password, setPassword] = useState(
    draft.archive.password === "••••••••" ? "" : draft.archive.password,
  );
  const [showPassword, setShowPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const patch = (key, value) => setDraft((d) => ({ ...d, [key]: value }));
  const nested = (key, field, value) =>
    setDraft((d) => ({ ...d, [key]: { ...d[key], [field]: value } }));
  const updateRow = (key, index, field, value) =>
    patch(
      key,
      draft[key].map((row, i) =>
        i === index ? { ...row, [field]: value } : row,
      ),
    );
  const field = (label, key, props = {}) => (
    <Field label={t(label)}>
      <input
        required
        value={draft[key]}
        onChange={(e) => patch(key, e.target.value)}
        {...props}
      />
    </Field>
  );
  const number = (label, key, field, min, max) => (
    <Field label={t(label)}>
      <input
        type="number"
        disabled={key === "retention" && draft.archive.kind === "none"}
        min={min}
        max={max}
        required
        value={draft[key][field]}
        onChange={(e) =>
          nested(
            key,
            field,
            e.target.value === "" ? "" : Number(e.target.value),
          )
        }
      />
    </Field>
  );
  const submit = async (e) => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      const input = {
        ...draft,
        retention:
          draft.archive.kind === "none"
            ? { keep_days: 0, keep_count: 0 }
            : draft.retention,
        schedule: mode === "simple" ? scheduleCron(simple) : draft.schedule,
        archive: {
          ...draft.archive,
          password:
            password ||
            (plan?.archive.password === "••••••••" ? "••••••••" : ""),
        },
        rclone_flags: splitArgs(flags),
      };
      await api(plan ? `/api/plans/${plan.id}` : "/api/plans", {
        method: plan ? "PUT" : "POST",
        body: JSON.stringify(input),
      });
      notify(t("saveSuccess"));
      await refresh(true);
      onClose();
    } catch (e) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title={plan ? `${t("edit")} · ${plan.name}` : t("newPlan")}
      description={t("sourcesTargetsHint")}
      onClose={onClose}
      closeLabel={t("close")}
      wide
    >
      <form onSubmit={submit}>
        <div className="modal-body">
          <ErrorNotice>{error}</ErrorNotice>
          <section className="form-section">
            <h3>{t("basics")}</h3>
            <div className="form-grid">
              {field("planName", "name", {
                maxLength: 80,
                autoFocus: true,
                placeholder: say("例如：家庭服务器", "e.g. Home server"),
              })}
              {field("timezone", "timezone", { list: "timezones" })}
              <datalist id="timezones">
                {[
                  "UTC",
                  "Asia/Shanghai",
                  "Asia/Hong_Kong",
                  "Europe/London",
                  "America/New_York",
                ].map((zone) => (
                  <option key={zone} value={zone} />
                ))}
              </datalist>
              <Field label={t("scheduleMode")}>
                <select value={mode} onChange={(e) => setMode(e.target.value)}>
                  <option value="simple">{t("simpleSchedule")}</option>
                  <option value="cron">Cron</option>
                </select>
              </Field>
              {mode === "simple" ? (
                <>
                  <Field label={t("scheduleFrequency")}>
                    <select
                      value={simple.kind}
                      onChange={(e) =>
                        setSimple({
                          ...simple,
                          kind: e.target.value,
                          interval: 1,
                        })
                      }
                    >
                      {[
                        "daily",
                        "weekly",
                        "monthly",
                        "every_hours",
                        "every_minutes",
                        "every_seconds",
                      ].map((kind) => (
                        <option key={kind} value={kind}>
                          {t(
                            {
                              every_hours: "everyHours",
                              every_minutes: "everyMinutes",
                              every_seconds: "everySeconds",
                            }[kind] || kind,
                          )}
                        </option>
                      ))}
                    </select>
                  </Field>
                  {["daily", "weekly", "monthly"].includes(simple.kind) ? (
                    <Field label={t("runAt")}>
                      <input
                        type="time"
                        required
                        value={simple.time || "02:00"}
                        onChange={(e) =>
                          setSimple({ ...simple, time: e.target.value })
                        }
                      />
                    </Field>
                  ) : (
                    <Field label={t("interval")}>
                      <input
                        type="number"
                        min="1"
                        max={simple.kind === "every_hours" ? 23 : 59}
                        required
                        value={simple.interval || 1}
                        onChange={(e) =>
                          setSimple({ ...simple, interval: e.target.value })
                        }
                      />
                    </Field>
                  )}
                  {simple.kind === "weekly" && (
                    <Field label={t("weekday")}>
                      <select
                        value={simple.weekday || "MON"}
                        onChange={(e) =>
                          setSimple({ ...simple, weekday: e.target.value })
                        }
                      >
                        {["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"].map(
                          (day, i) => (
                            <option key={day} value={day}>
                              {t(
                                [
                                  "monday",
                                  "tuesday",
                                  "wednesday",
                                  "thursday",
                                  "friday",
                                  "saturday",
                                  "sunday",
                                ][i],
                              )}
                            </option>
                          ),
                        )}
                      </select>
                    </Field>
                  )}
                  {simple.kind === "monthly" && (
                    <Field label={t("monthday")} hint={t("monthdayHint")}>
                      <input
                        type="number"
                        min="1"
                        max="31"
                        required
                        value={simple.monthday || 1}
                        onChange={(e) =>
                          setSimple({ ...simple, monthday: e.target.value })
                        }
                      />
                    </Field>
                  )}
                  <div className="schedule-preview span-2">
                    <ClockLabel />
                    <code>{scheduleCron(simple)}</code>
                  </div>
                </>
              ) : (
                field("schedule", "schedule", { maxLength: 256 })
              )}
            </div>
            <Toggle
              label={t("enabled")}
              hint={t("enabledHint")}
              checked={draft.enabled}
              onChange={(value) => patch("enabled", value)}
            />
          </section>
          <section className="form-section">
            <h3>{t("sourcesTargets")}</h3>
            {[
              ["sources", "folders", Folder],
              ["remotes", "remoteTargets", Globe2],
            ].map(([key, label, Icon]) => (
              <div className="repeat-group" key={key}>
                <div className="repeat-heading">
                  <strong>
                    <Icon size={16} />
                    {t(label)}
                  </strong>
                  <Button
                    onClick={() =>
                      patch(key, [
                        ...draft[key],
                        key === "sources"
                          ? { name: "", path: "" }
                          : { name: remotes[0]?.name || "", directory: "/" },
                      ])
                    }
                  >
                    <Plus size={14} />
                    {t("add")}
                  </Button>
                </div>
                {draft[key].map((row, index) => (
                  <div className="repeat-row" key={index}>
                    <Field
                      label={t(key === "sources" ? "label" : "remoteName")}
                    >
                      {key === "remotes" ? (
                        <select
                          required
                          value={row.name}
                          onChange={(e) =>
                            updateRow(key, index, "name", e.target.value)
                          }
                        >
                          <option value="">{t("remoteName")}</option>
                          {[
                            ...new Set(
                              [...remotes.map((r) => r.name), row.name].filter(
                                Boolean,
                              ),
                            ),
                          ].map((name) => (
                            <option key={name}>{name}</option>
                          ))}
                        </select>
                      ) : (
                        <input
                          required
                          value={row.name}
                          maxLength={80}
                          onChange={(e) =>
                            updateRow(key, index, "name", e.target.value)
                          }
                        />
                      )}
                    </Field>
                    <Field label={t(key === "sources" ? "path" : "remoteDir")}>
                      <input
                        required
                        value={row[key === "sources" ? "path" : "directory"]}
                        onChange={(e) =>
                          updateRow(
                            key,
                            index,
                            key === "sources" ? "path" : "directory",
                            e.target.value,
                          )
                        }
                        placeholder={key === "sources" ? "/data" : "/backups"}
                      />
                    </Field>
                    <IconButton
                      icon={Trash2}
                      label={t("remove")}
                      disabled={draft[key].length === 1}
                      onClick={() =>
                        patch(
                          key,
                          draft[key].filter((_, i) => i !== index),
                        )
                      }
                    />
                  </div>
                ))}
              </div>
            ))}
          </section>
          <section className="form-section">
            <h3>{t("archiveEncryption")}</h3>
            <div className="form-grid">
              <Field label={t("archiveType")}>
                <select
                  value={draft.archive.kind}
                  onChange={(e) => nested("archive", "kind", e.target.value)}
                >
                  <option value="7z">{t("secureArchive")}</option>
                  <option value="zip">{t("compatibleArchive")}</option>
                  <option value="none">{t("nativeDirectory")}</option>
                </select>
              </Field>
              <Field label={t("fileSuffix")}>
                <input
                  required
                  value={draft.archive.suffix}
                  onChange={(e) => nested("archive", "suffix", e.target.value)}
                />
              </Field>
              <Field
                label={t("archivePassword")}
                hint={t(
                  draft.archive.kind === "7z"
                    ? "secureArchiveHint"
                    : draft.archive.kind === "zip"
                      ? "compatibleArchiveHint"
                      : "nativeDirectoryHint",
                )}
              >
                <span className="password-field">
                  <input
                    type={showPassword ? "text" : "password"}
                    value={password}
                    disabled={draft.archive.kind === "none"}
                    placeholder={
                      plan?.archive.password === "••••••••"
                        ? t("configuredSecret")
                        : ""
                    }
                    autoComplete="new-password"
                    onChange={(e) => setPassword(e.target.value)}
                  />
                  <IconButton
                    icon={showPassword ? EyeOff : Eye}
                    label={t(showPassword ? "hidePassword" : "showPassword")}
                    onClick={async () => {
                      if (
                        !showPassword &&
                        plan?.archive.password === "••••••••" &&
                        !password
                      ) {
                        if (!status.authentication_enabled) {
                          setError(t("passwordRevealNeedsAuth"));
                          return;
                        }
                        try {
                          const result = await api(
                            `/api/plans/${plan.id}/archive-password`,
                            { method: "POST" },
                          );
                          setPassword(result.password);
                        } catch (e) {
                          setError(e.message);
                          return;
                        }
                      }
                      setShowPassword(!showPassword);
                    }}
                  />
                </span>
              </Field>
              <Field label={t("passwordHint")} hint={t("passwordHintHelp")}>
                <input
                  value={draft.archive.password_hint || ""}
                  onChange={(e) =>
                    nested("archive", "password_hint", e.target.value)
                  }
                />
              </Field>
            </div>
          </section>
          <details className="form-section advanced">
            <summary>
              {say(
                "保留策略、重试与高级选项",
                "Retention, retries & advanced options",
              )}
            </summary>
            <div className="form-grid">
              {number("keepDays", "retention", "keep_days", 0, 36500)}
              {number("keepCount", "retention", "keep_count", 0, 100000)}
              <p className="field-help span-2">
                {say(
                  "填 0 表示不按该条件清理备份。原生目录模式不使用归档保留策略。",
                  "Use 0 to disable retention by that condition. Native-directory backups do not use archive retention.",
                )}
              </p>
              <Field label={t("maxAttempts")} hint={t("retryCountHint")}>
                <input
                  type="number"
                  min="0"
                  max="19"
                  required
                  value={Math.max(0, draft.retry.max_attempts - 1)}
                  onChange={(e) =>
                    nested("retry", "max_attempts", Number(e.target.value) + 1)
                  }
                />
              </Field>
              <Field label={t("backoff")}>
                <select
                  value={draft.retry.backoff}
                  onChange={(e) => nested("retry", "backoff", e.target.value)}
                >
                  <option value="exponential">{t("exponential")}</option>
                  <option value="fixed">{t("fixed")}</option>
                </select>
              </Field>
              {number(
                "initialDelay",
                "retry",
                "initial_delay_seconds",
                0,
                86400,
              )}
              {number("maxDelay", "retry", "max_delay_seconds", 0, 86400)}
              {field("remoteCheckConcurrency", "remote_check_concurrency", {
                type: "number",
                min: 1,
                max: 32,
                onChange: (e) =>
                  patch("remote_check_concurrency", Number(e.target.value)),
              })}
              {field("uploadConcurrency", "upload_concurrency", {
                type: "number",
                min: 1,
                max: 8,
                onChange: (e) =>
                  patch("upload_concurrency", Number(e.target.value)),
              })}
              <Field
                label={t("rcloneFlags")}
                hint={t("flagsHint")}
                className="span-2"
              >
                <input
                  value={flags}
                  onChange={(e) => setFlags(e.target.value)}
                  placeholder="--transfers 4"
                />
              </Field>
            </div>
          </details>
        </div>
        <footer className="modal-footer">
          <Button onClick={onClose} disabled={busy}>
            {t("cancel")}
          </Button>
          <Button type="submit" variant="primary" busy={busy}>
            {t("savePlan")}
          </Button>
        </footer>
      </form>
    </Modal>
  );
}
function ClockLabel() {
  return <span aria-hidden="true">Cron</span>;
}
