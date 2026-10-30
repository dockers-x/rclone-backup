import { useEffect, useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { useApp } from "../App";
import {
  Button,
  Field,
  IconButton,
  Modal,
  ErrorNotice,
  Loading,
} from "../components/ui";
import { providerFieldIsSecret } from "../lib/helpers";
import { api } from "../lib/api";
export function providerValue(option, value) {
  const type = String(option?.Type ?? option?.type ?? "").toLowerCase();
  if (type === "tristate" && value && typeof value === "object")
    return (value.Valid ?? value.valid ?? false)
      ? String(value.Value ?? value.value ?? "")
      : "";
  if (Array.isArray(value))
    return value.join(type === "spaceseplist" ? " " : ",");
  if (value === null || value === undefined || typeof value === "object")
    return "";
  return String(value);
}
export function RemoteModal({ remote, onClose }) {
  const { t, say, notify, refresh } = useApp();
  const [providers, setProviders] = useState(null);
  const [type, setType] = useState(remote?.type || "");
  const [name, setName] = useState(remote?.name || "");
  const [values, setValues] = useState(remote?.parameters || {});
  const [error, setError] = useState("");
  const [result, setResult] = useState("");
  const [flow, setFlow] = useState(null);
  const [answer, setAnswer] = useState("");
  const [busy, setBusy] = useState(false);
  const [extra, setExtra] = useState([]);
  useEffect(() => {
    let alive = true;
    api("/api/rclone/providers")
      .then((data) => {
        if (alive) setProviders(data.providers || []);
      })
      .catch((e) => {
        if (alive) setError(e.message);
      });
    return () => {
      alive = false;
    };
  }, []);
  const provider = providers?.find(
    (p) => (p.Name || p.name || p.Prefix || p.prefix) === type,
  );
  const knownOptions = provider?.Options || provider?.options || [];
  const knownNames = new Set(knownOptions.map((o) => o.Name || o.name));
  const extraNames = [
    ...new Set([
      ...Object.keys(remote?.parameters || {}),
      ...(remote?.configured_secrets || []),
    ]),
  ].filter((name) => !knownNames.has(name));
  const options = [
    ...knownOptions,
    ...extraNames.map((Name) => ({ Name, Advanced: true })),
  ];
  const secrets = new Set(remote?.configured_secrets || []);
  const visible = options.filter(
    (option) =>
      !(option.Advanced ?? option.advanced) ||
      extra.includes(option.Name || option.name) ||
      Object.hasOwn(values, option.Name || option.name) ||
      secrets.has(option.Name || option.name),
  );
  const hidden = options.filter((option) => !visible.includes(option));
  async function submit(event) {
    event.preventDefault();
    setBusy(true);
    setError("");
    setResult("");
    try {
      if (
        type === "s3" &&
        values.endpoint &&
        !/^https?:\/\//i.test(values.endpoint)
      )
        throw new Error(t("endpointNeedsScheme"));
      const parameters = {};
      for (const [key, value] of Object.entries(values)) {
        const option = options.find((o) => (o.Name || o.name) === key);
        const original = providerValue(
          option,
          remote?.parameters?.[key] ?? option?.Default ?? option?.default,
        );
        if (value !== original && !(secrets.has(key) && value === ""))
          parameters[key] = value;
      }
      const payload = remote
        ? { parameters }
        : {
            name: name.trim(),
            type,
            parameters,
            ...(flow
              ? { state: flow.State || flow.state, result: answer }
              : {}),
          };
      const response = await api(
        remote
          ? `/api/rclone/remotes/${encodeURIComponent(name)}`
          : "/api/rclone/remotes",
        { method: remote ? "PUT" : "POST", body: JSON.stringify(payload) },
      );
      if (
        response.State ||
        response.state ||
        response.Option ||
        response.option
      ) {
        setFlow(response);
        setAnswer(String(response.Result || response.result || ""));
        return;
      }
      const text = t(
        response.verified ? "remoteSavedVerified" : "remoteSavedUnverified",
      );
      setResult(text);
      notify(text, !response.verified);
      await refresh(true);
      if (response.verified) onClose();
    } catch (e) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }
  const question = flow?.Option || flow?.option;
  return (
    <Modal
      title={remote ? t("remoteUpdate") : t("configureStorage")}
      description={t("storageIntro")}
      onClose={onClose}
      closeLabel={t("close")}
      wide
    >
      <form onSubmit={submit}>
        <div className="modal-body">
          <ErrorNotice>{error}</ErrorNotice>
          {result && (
            <div className="setup-notice" role="status">
              {result}
            </div>
          )}
          {!providers ? (
            <Loading />
          ) : (
            <>
              <div className="form-grid">
                <Field label={t("rcloneAlias")}>
                  <input
                    required
                    maxLength={80}
                    readOnly={Boolean(remote)}
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    autoFocus
                  />
                </Field>
                <Field label={t("provider")}>
                  <select
                    required
                    disabled={Boolean(remote)}
                    value={type}
                    onChange={(e) => {
                      setType(e.target.value);
                      setValues({});
                      setFlow(null);
                      setExtra([]);
                    }}
                  >
                    <option value="">{t("provider")}</option>
                    {providers
                      .slice()
                      .sort((a, b) =>
                        (a.Description || a.Name || "").localeCompare(
                          b.Description || b.Name || "",
                        ),
                      )
                      .map((p) => {
                        const key = p.Name || p.name || p.Prefix || p.prefix;
                        return (
                          <option key={key} value={key}>
                            {p.Description || p.description || key} ({key})
                          </option>
                        );
                      })}
                  </select>
                </Field>
              </div>
              {provider && (
                <div className="provider-fields">
                  {visible.map((option) => {
                    const key = option.Name || option.name;
                    return (
                      <ProviderField
                        key={`${type}:${key}`}
                        option={option}
                        value={
                          values[key] ??
                          (secrets.has(key)
                            ? ""
                            : providerValue(
                                option,
                                option.Default ?? option.default,
                              ))
                        }
                        onChange={(value) =>
                          setValues({ ...values, [key]: value })
                        }
                        secret={secrets.has(key)}
                        editing={Boolean(remote)}
                        values={values}
                      />
                    );
                  })}
                </div>
              )}
              {hidden.length > 0 && (
                <Field label={t("addConfigurationItem")}>
                  <select
                    value=""
                    onChange={(e) => setExtra([...extra, e.target.value])}
                  >
                    <option value="">{t("chooseConfigurationItem")}</option>
                    {hidden.map((o) => (
                      <option key={o.Name || o.name} value={o.Name || o.name}>
                        {o.Name || o.name}
                      </option>
                    ))}
                  </select>
                </Field>
              )}
              {question && (
                <section className="form-section">
                  <h3>{t("remoteNeedsInput")}</h3>
                  <p className="field-help">{question.Help || question.help}</p>
                  <ProviderField
                    option={question}
                    value={answer}
                    onChange={setAnswer}
                    values={values}
                  />
                </section>
              )}
            </>
          )}
        </div>
        <footer className="modal-footer">
          <Button onClick={onClose}>{t("close")}</Button>
          <Button type="submit" variant="primary" busy={busy} disabled={!type}>
            {t("saveAndTest")}
          </Button>
        </footer>
      </form>
    </Modal>
  );
}
export function ProviderField({
  option,
  value,
  onChange,
  secret = false,
  editing = false,
  values = {},
}) {
  const { t, language } = useApp();
  const [show, setShow] = useState(false);
  const key = option.Name || option.name;
  const help = option.Help || option.help || "";
  const required = !editing && (option.Required ?? option.required ?? false);
  const password = secret || providerFieldIsSecret(option, key);
  const examples = (option.Examples || option.examples || []).filter((e) => {
    const provider = e.Provider || e.provider;
    if (!provider) return true;
    const current = values.provider || "";
    return provider.startsWith("!")
      ? !provider.slice(1).split(",").includes(current)
      : provider.split(",").includes(current);
  });
  const optionType = String(option.Type || option.type || "").toLowerCase();
  const exclusive = option.Exclusive ?? option.exclusive;
  const select =
    !password && (exclusive || ["bool", "tristate"].includes(optionType));
  const choices = examples.length
    ? examples
    : [
        { Value: "false", Help: "false" },
        { Value: "true", Help: "true" },
      ];
  return (
    <Field
      label={providerFieldLabel(key, language) + (required ? " *" : "")}
      hint={secret ? t("configuredSecret") : help.split("\n")[0]}
    >
      {select ? (
        <select
          required={required}
          value={value}
          onChange={(e) => onChange(e.target.value)}
        >
          {!required && <option value="">—</option>}
          {value &&
            !choices.some((e) => String(e.Value ?? e.value) === value) && (
              <option value={value}>{value}</option>
            )}
          {choices.map((e, i) => (
            <option key={i} value={String(e.Value ?? e.value)}>
              {e.Help || e.help || String(e.Value ?? e.value)}
            </option>
          ))}
        </select>
      ) : (
        <span className={password ? "password-field" : "input-with-options"}>
          <input
            required={required}
            type={password && !show ? "password" : "text"}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={secret ? t("configuredSecret") : ""}
            autoComplete={password ? "new-password" : "off"}
            list={!password && examples.length ? `provider-${key}` : undefined}
          />
          {password && (
            <IconButton
              icon={show ? EyeOff : Eye}
              label={t(show ? "hidePassword" : "showPassword")}
              onClick={() => setShow(!show)}
            />
          )}
          {!password && examples.length > 0 && (
            <datalist id={`provider-${key}`}>
              {examples.map((e, i) => (
                <option key={i} value={String(e.Value ?? e.value)}>
                  {e.Help || e.help}
                </option>
              ))}
            </datalist>
          )}
        </span>
      )}
    </Field>
  );
}

export function providerFieldLabel(name, language) {
  const zh = {
    url: "网址",
    vendor: "WebDAV 类型",
    user: "用户名",
    pass: "密码",
    provider: "S3 服务商",
    access_key_id: "Access Key ID",
    secret_access_key: "Secret Access Key",
    region: "区域",
    endpoint: "Endpoint",
    location_constraint: "存储区域约束",
    acl: "访问权限",
  };
  const en = {
    url: "URL",
    vendor: "WebDAV vendor",
    user: "Username",
    pass: "Password",
    provider: "S3 provider",
    region: "Region",
    location_constraint: "Location constraint",
    acl: "ACL",
  };
  return (language === "zh" ? zh : en)[name] || name;
}
