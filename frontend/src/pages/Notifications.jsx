import { useEffect, useState } from "react";
import {
  Bell,
  Plus,
  ChevronDown,
  ChevronUp,
  Trash2,
  Send,
  Copy,
  FileText,
  Save,
} from "lucide-react";
import { useApp } from "../App";
import {
  Button,
  Field,
  IconButton,
  Empty,
  Toggle,
  Tabs,
  ErrorNotice,
  Badge,
} from "../components/ui";
import { api } from "../lib/api";
import { renderTemplateValue } from "../lib/helpers";
function blankTemplate(language) {
  return Object.fromEntries(
    ["start", "success", "failure"].map((event, i) => [
      event,
      {
        title:
          language === "zh"
            ? `{{plan_name}} · ${["备份开始", "备份成功", "备份失败"][i]}`
            : `{{plan_name}} · ${["Backup started", "Backup succeeded", "Backup failed"][i]}`,
        body: "{{content}}\n{{time}}\n{{backup_size}}",
      },
    ]),
  );
}
function builtin(t) {
  return {
    id: "",
    name: t("builtInTemplate"),
    language: "en",
    ...Object.fromEntries(
      ["start", "success", "failure"].map((event, i) => [
        event,
        {
          title: `{{plan_name}} Backup ${["Start", "Success", "Failed"][i]}`,
          body: "{{content}}",
        },
      ]),
    ),
  };
}
export function NotificationsPage() {
  const { notifications, t, say, notify, refresh, confirm } = useApp();
  const [targets, setTargets] = useState(() =>
    structuredClone(notifications.config?.targets || []),
  );
  const [expanded, setExpanded] = useState(null);
  const [type, setType] = useState("ping");
  const [busy, setBusy] = useState(false);
  const [testId, setTestId] = useState(null);
  const [error, setError] = useState("");
  const [dirty, setDirty] = useState(false);
  useDraftWarning(dirty);
  const templates = notifications.config?.templates || [];
  const update = (id, patch) => {
    setTargets((rows) =>
      rows.map((row) => (row.id === id ? { ...row, ...patch } : row)),
    );
    setDirty(true);
  };
  const config = () => ({ ...notifications.config, targets });
  async function save(event) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      const response = await api("/api/notifications", {
        method: "PUT",
        body: JSON.stringify({ config: config() }),
      });
      setTargets(structuredClone(response.config?.targets || []));
      setDirty(false);
      notify(t("notificationSaved"));
      await refresh(true);
    } catch (e) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }
  const add = () => {
    const id = crypto.randomUUID();
    const channel = type.startsWith("serverchan") ? "serverchan" : type;
    setTargets([
      ...targets,
      {
        id,
        name: type,
        enabled: true,
        on_start: false,
        on_success: true,
        on_failure: true,
        template_id: "",
        type: channel,
        config:
          type === "email"
            ? {
                host: "",
                port: 587,
                security: "starttls",
                from: "",
                username: "",
                password: "",
                to: "",
              }
            : type === "ntfy"
              ? { server: "https://ntfy.sh", topic: "", token: "" }
              : channel === "serverchan"
                ? {
                    channel: type === "serverchan-app" ? "app" : "wechat",
                    send_key: "",
                  }
                : {
                    completion_url: "",
                    start_url: "",
                    success_url: "",
                    failure_url: "",
                  },
      },
    ]);
    setExpanded(id);
    setDirty(true);
  };
  return (
    <form onSubmit={save}>
      <ErrorNotice>{error}</ErrorNotice>
      <div className="panel notification-panel">
        <div className="panel-toolbar">
          <span className="muted">{targets.length} / 32</span>
          <div className="inline-actions">
            <select
              aria-label={t("notificationType")}
              value={type}
              onChange={(e) => setType(e.target.value)}
            >
              {[
                ["ping", "Ping"],
                ["email", "Email"],
                ["serverchan-app", t("serverChanApp")],
                ["serverchan-wechat", t("serverChanWechat")],
                ["ntfy", "ntfy"],
              ].map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
            <Button disabled={targets.length >= 32} onClick={add}>
              <Plus size={16} />
              {t("addNotification")}
            </Button>
          </div>
        </div>
        {!targets.length ? (
          <Empty
            icon={Bell}
            title={t("noNotificationTargets")}
            hint={t("noNotificationTargetsHint")}
          />
        ) : (
          <div className="notification-list">
            {targets.map((target) => (
              <article className="notification-target" key={target.id}>
                <div className="notification-row">
                  <span className="object-icon small">
                    <Bell size={18} />
                  </span>
                  <button
                    type="button"
                    className="notification-name"
                    onClick={() =>
                      setExpanded(expanded === target.id ? null : target.id)
                    }
                    aria-expanded={expanded === target.id}
                    aria-controls={`target-${target.id}`}
                  >
                    <strong>{target.name}</strong>
                    <small>
                      {target.type} ·{" "}
                      {templates.find((t) => t.id === target.template_id)
                        ?.name || t("builtInTemplate")}
                    </small>
                  </button>
                  <Toggle
                    label={`${t("enableNotification")} ${target.name}`}
                    checked={target.enabled}
                    onChange={(value) => update(target.id, { enabled: value })}
                  />
                  <IconButton
                    icon={expanded === target.id ? ChevronUp : ChevronDown}
                    label={t(expanded === target.id ? "collapse" : "edit")}
                    onClick={() =>
                      setExpanded(expanded === target.id ? null : target.id)
                    }
                  />
                </div>
                {expanded === target.id && (
                  <div
                    className="notification-editor"
                    id={`target-${target.id}`}
                  >
                    <div className="form-grid">
                      <Field label={t("notificationName")}>
                        <input
                          required
                          maxLength={80}
                          value={target.name}
                          onChange={(e) =>
                            update(target.id, { name: e.target.value })
                          }
                        />
                      </Field>
                      <Field label={t("selectedTemplate")}>
                        <select
                          value={target.template_id || ""}
                          onChange={(e) =>
                            update(target.id, { template_id: e.target.value })
                          }
                        >
                          <option value="">{t("builtInTemplate")}</option>
                          {templates.map((template) => (
                            <option key={template.id} value={template.id}>
                              {template.name}
                            </option>
                          ))}
                        </select>
                      </Field>
                      <NotificationFields
                        target={target}
                        onChange={(config) => update(target.id, { config })}
                      />
                    </div>
                    <fieldset className="event-options">
                      <legend>{t("notificationEvents")}</legend>
                      {["start", "success", "failure"].map((event) => (
                        <label key={event}>
                          <input
                            type="checkbox"
                            checked={target[`on_${event}`]}
                            onChange={(e) =>
                              update(target.id, {
                                [`on_${event}`]: e.target.checked,
                              })
                            }
                          />
                          {t(
                            {
                              start: "onStart",
                              success: "onSuccess",
                              failure: "onFailure",
                            }[event],
                          )}
                        </label>
                      ))}
                    </fieldset>
                    <div className="editor-actions">
                      <Button
                        variant="danger-quiet"
                        onClick={() =>
                          confirm({
                            title: `${t("removeNotification")} · ${target.name}`,
                            description: say(
                              "保存通知配置后生效。",
                              "The removal takes effect when you save.",
                            ),
                            onConfirm: async () => {
                              setTargets((rows) =>
                                rows.filter((row) => row.id !== target.id),
                              );
                              setDirty(true);
                            },
                          })
                        }
                      >
                        <Trash2 size={15} />
                        {t("removeNotification")}
                      </Button>
                      <Button
                        busy={testId === target.id}
                        onClick={async () => {
                          setTestId(target.id);
                          try {
                            await api("/api/notifications/test", {
                              method: "POST",
                              body: JSON.stringify({
                                target_id: target.id,
                                config: config(),
                              }),
                            });
                            notify(t("notificationTestSuccess"));
                          } catch (e) {
                            setError(e.message);
                            notify(e.message, true);
                          } finally {
                            setTestId(null);
                          }
                        }}
                      >
                        <Send size={15} />
                        {t("sendTest")}
                      </Button>
                    </div>
                  </div>
                )}
              </article>
            ))}
          </div>
        )}
      </div>
      <div className="save-bar">
        <span className="muted" role="status">
          {dirty
            ? say("有未保存的更改", "You have unsaved changes")
            : say("所有更改已保存", "All changes saved")}
        </span>
        <Button type="submit" variant="primary" busy={busy}>
          <Save size={16} />
          {t("saveNotifications")}
        </Button>
      </div>
    </form>
  );
}
function NotificationFields({ target, onChange }) {
  const { t } = useApp();
  const config = target.config;
  const input = (label, key, type = "text", required = false, extra = {}) => (
    <Field key={key} label={label}>
      <input
        type={type}
        required={required}
        value={config[key] ?? ""}
        onChange={(e) =>
          onChange({
            ...config,
            [key]: type === "number" ? Number(e.target.value) : e.target.value,
          })
        }
        autoComplete={type === "password" ? "new-password" : undefined}
        {...extra}
      />
    </Field>
  );
  if (target.type === "email")
    return (
      <>
        {input(t("smtpHost"), "host", "text", true)}
        {input(t("smtpPort"), "port", "number", true, { min: 1, max: 65535 })}
        <Field label={t("smtpSecurity")}>
          <select
            value={config.security || "starttls"}
            onChange={(e) => onChange({ ...config, security: e.target.value })}
          >
            <option value="starttls">{t("smtpStarttls")}</option>
            <option value="tls">TLS</option>
          </select>
        </Field>
        {input(t("smtpFrom"), "from", "email", true)}
        {input(t("smtpUsername"), "username")}
        {input(t("smtpPassword"), "password", "password")}
        {input(t("recipient"), "to", "email", true)}
      </>
    );
  if (target.type === "ntfy")
    return (
      <>
        {input("ntfy Server", "server", "url", true)}
        {input("Topic", "topic", "text", true, { pattern: "[A-Za-z0-9_-]+" })}
        {input("Token", "token", "password")}
      </>
    );
  if (target.type === "serverchan")
    return input("SendKey", "send_key", "password", true);
  return [
    ["completionUrl", "completion_url"],
    ["startUrl", "start_url"],
    ["successUrl", "success_url"],
    ["failureUrl", "failure_url"],
  ].map(([label, key]) => input(t(label), key, "password"));
}
export function TemplatesPage() {
  const { notifications, t, say, language, notify, refresh, confirm } =
    useApp();
  const [templates, setTemplates] = useState(() =>
    structuredClone(notifications.config?.templates || []),
  );
  const [selected, setSelected] = useState("");
  const [event, setEvent] = useState(1);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [dirty, setDirty] = useState(false);
  const targets = notifications.config?.targets || [];
  useDraftWarning(dirty);
  const template = templates.find((row) => row.id === selected) || builtin(t);
  const eventKey = ["start", "success", "failure"][event];
  const message = template[eventKey];
  const patch = (patch) => {
    setTemplates((rows) =>
      rows.map((row) => (row.id === selected ? { ...row, ...patch } : row)),
    );
    setDirty(true);
  };
  const add = () => {
    const id = crypto.randomUUID();
    setTemplates([
      ...templates,
      { id, name: t("createTemplate"), language, ...blankTemplate(language) },
    ]);
    setSelected(id);
    setDirty(true);
  };
  const used = targets.filter((target) => target.template_id === selected);
  async function save(e) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      const result = await api("/api/notification-templates", {
        method: "PUT",
        body: JSON.stringify({ templates }),
      });
      setTemplates(structuredClone(result.config?.templates || []));
      setDirty(false);
      notify(t("templateSaved"));
      await refresh(true);
    } catch (e) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <form onSubmit={save}>
      <ErrorNotice>{error}</ErrorNotice>
      <div className="template-layout">
        <aside className="template-sidebar">
          <div className="template-sidebar-title">
            <strong>{say("模板库", "Library")}</strong>
            <IconButton
              icon={Plus}
              label={t("createTemplate")}
              disabled={templates.length >= 32}
              onClick={add}
            />
          </div>
          {[builtin(t), ...templates].map((row) => (
            <button
              key={row.id}
              type="button"
              className={`template-item ${row.id === selected ? "selected" : ""}`}
              onClick={() => setSelected(row.id)}
            >
              <FileText size={17} />
              <span>
                <strong>{row.name}</strong>
                <small>
                  {row.id
                    ? row.language === "zh"
                      ? "中文"
                      : "English"
                    : t("readOnly")}
                </small>
              </span>
            </button>
          ))}
        </aside>
        <section className="template-editor">
          <div className="editor-heading">
            <h2>{template.name}</h2>
            <div className="inline-actions">
              <IconButton
                icon={Copy}
                label={t("duplicateTemplate")}
                disabled={templates.length >= 32}
                onClick={() => {
                  const id = crypto.randomUUID();
                  setTemplates([
                    ...templates,
                    {
                      ...structuredClone(template),
                      id,
                      name: Array.from(
                        `${template.name} · ${t("duplicateTemplate")}`,
                      )
                        .slice(0, 80)
                        .join(""),
                    },
                  ]);
                  setSelected(id);
                  setDirty(true);
                }}
              />
              {selected && (
                <IconButton
                  icon={Trash2}
                  label={t("deleteTemplate")}
                  onClick={() => {
                    if (used.length) {
                      setError(
                        t("templateDeleteBlocked").replace(
                          "{targets}",
                          used.map((target) => target.name).join(", "),
                        ),
                      );
                      return;
                    }
                    confirm({
                      title: t("templateDeleteConfirm").replace(
                        "{name}",
                        template.name,
                      ),
                      description: t("templateDeleteDescription"),
                      onConfirm: async () => {
                        setTemplates((rows) =>
                          rows.filter((row) => row.id !== selected),
                        );
                        setSelected("");
                        setDirty(true);
                      },
                    });
                  }}
                />
              )}
            </div>
          </div>
          {selected && (
            <div className="form-grid">
              <Field label={t("templateName")}>
                <input
                  required
                  maxLength={80}
                  value={template.name}
                  onChange={(e) => patch({ name: e.target.value })}
                />
              </Field>
              <Field
                label={t("templateLanguage")}
                hint={t("templateLanguageHint")}
              >
                <select
                  value={template.language}
                  onChange={(e) => patch({ language: e.target.value })}
                >
                  <option value="zh">中文</option>
                  <option value="en">English</option>
                </select>
              </Field>
            </div>
          )}
          <Tabs
            tabs={[
              t("templateStart"),
              t("templateSuccess"),
              t("templateFailure"),
            ]}
            label={t("notificationEvents")}
            value={event}
            onChange={setEvent}
            panels={[null, null, null]}
          />
          <div className="template-fields">
            <Field label={t("templateTitle")}>
              <input
                value={message.title}
                maxLength={200}
                required
                readOnly={!selected}
                onChange={(e) =>
                  patch({ [eventKey]: { ...message, title: e.target.value } })
                }
              />
            </Field>
            <Field label={t("templateBody")}>
              <textarea
                value={message.body}
                maxLength={16000}
                required
                rows={5}
                readOnly={!selected}
                onChange={(e) =>
                  patch({ [eventKey]: { ...message, body: e.target.value } })
                }
              />
            </Field>
            <div className="variables">
              <span>{t("templatePlaceholders")}</span>
              {["plan_name", "event", "content", "time", "backup_size"].map(
                (variable) => (
                  <code key={variable}>{"{{" + variable + "}}"}</code>
                ),
              )}
            </div>
          </div>
          <div className="template-preview">
            <span className="preview-label">
              {t("templatePreview")} · {say("示例数据", "Sample data")}
            </span>
            <strong>
              {renderTemplateValue(message.title, eventKey, template.language)}
            </strong>
            <p>
              {renderTemplateValue(message.body, eventKey, template.language)}
            </p>
          </div>
        </section>
      </div>
      <div className="save-bar">
        <span className="muted" role="status">
          {dirty
            ? say("有未保存的更改", "You have unsaved changes")
            : say("所有更改已保存", "All changes saved")}
        </span>
        <Button type="submit" variant="primary" busy={busy}>
          <Save size={16} />
          {t("saveTemplates")}
        </Button>
      </div>
    </form>
  );
}

function useDraftWarning(dirty) {
  useEffect(() => {
    if (!dirty) return;
    const warn = (event) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);
}
