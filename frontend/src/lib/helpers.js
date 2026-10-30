export function parseRunProgress(run) {
  const progress = {
    phase: run.status,
    phaseAt: null,
    updatedAt: null,
    targets: new Map(),
    log: [],
  };
  for (const line of (run.log || "").split("\n")) {
    const marker = line.indexOf("@event ");
    if (marker < 0) {
      if (line) progress.log.push(line);
      continue;
    }
    try {
      const event = JSON.parse(line.slice(marker + 7));
      const eventTime = Date.parse(event.at);
      if (
        Number.isFinite(eventTime) &&
        (!progress.updatedAt || eventTime > Date.parse(progress.updatedAt))
      )
        progress.updatedAt = event.at;
      if (event.kind === "phase") {
        progress.phase = event.phase;
        progress.phaseAt = event.at || progress.phaseAt;
      }
      if (event.kind === "target") {
        const key = `${event.name}\u0000${event.directory || ""}`;
        const previous = progress.targets.get(key);
        const startedAt =
          event.status === "pending"
            ? null
            : ["checking", "uploading"].includes(event.status)
              ? event.at
              : previous?.startedAt;
        progress.targets.set(key, { ...event, startedAt });
      }
    } catch {
      progress.log.push(line);
    }
  }
  return progress;
}

export function normalizeRunProgress(run, progress) {
  if (!["cancelled", "interrupted", "timed_out"].includes(run.status))
    return progress;
  const unfinished = ["pending", "checking", "uploading"];
  const finishedAt = run.finished_at || progress.updatedAt;
  return {
    ...progress,
    phase: run.status,
    updatedAt: finishedAt,
    targets: new Map(
      [...progress.targets].map(([key, target]) => [
        key,
        unfinished.includes(target.status)
          ? { ...target, status: run.status, at: finishedAt || target.at }
          : target,
      ]),
    ),
  };
}

export function parseSimpleSchedule(schedule) {
  const fields = schedule.trim().split(/\s+/);
  const normalized =
    fields.length === 5
      ? ["0", ...fields]
      : fields.length === 6
        ? fields
        : null;
  if (!normalized) return null;
  const [second, minute, hour, monthday, month, weekday] = normalized;
  if (month !== "*") return null;
  if (
    /^(?:[0-9]|[1-5]\d)$/.test(minute) &&
    /^(?:[0-9]|1\d|2[0-3])$/.test(hour)
  ) {
    const time = `${hour.padStart(2, "0")}:${minute.padStart(2, "0")}`;
    if (second === "0" && monthday === "*" && weekday === "*")
      return { kind: "daily", time };
    if (
      second === "0" &&
      monthday === "*" &&
      /^(MON|TUE|WED|THU|FRI|SAT|SUN)$/.test(weekday)
    )
      return { kind: "weekly", time, weekday };
    if (
      second === "0" &&
      /^([1-9]|[12]\d|3[01])$/.test(monthday) &&
      weekday === "*"
    )
      return { kind: "monthly", time, monthday };
  }
  let match = second.match(/^0\/([1-9]|[1-5]\d)$/);
  if (
    match &&
    minute === "*" &&
    hour === "*" &&
    monthday === "*" &&
    weekday === "*"
  )
    return { kind: "every_seconds", interval: match[1] };
  match = minute.match(/^0\/([1-9]|[1-5]\d)$/);
  if (
    second === "0" &&
    match &&
    hour === "*" &&
    monthday === "*" &&
    weekday === "*"
  )
    return { kind: "every_minutes", interval: match[1] };
  match = hour.match(/^0\/([1-9]|1\d|2[0-3])$/);
  if (
    second === "0" &&
    minute === "0" &&
    match &&
    monthday === "*" &&
    weekday === "*"
  )
    return { kind: "every_hours", interval: match[1] };
  return null;
}

export function splitArgs(value) {
  const result = [];
  let current = "";
  let quote = null;
  let escaped = false;
  for (const char of value.trim()) {
    if (escaped) {
      current += char;
      escaped = false;
      continue;
    }
    if (char === "\\") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (char === quote) quote = null;
      else current += char;
      continue;
    }
    if (char === "'" || char === '"') {
      quote = char;
      continue;
    }
    if (/\s/.test(char)) {
      if (current) {
        result.push(current);
        current = "";
      }
    } else current += char;
  }
  if (current) result.push(current);
  return result;
}

export function joinArgs(values) {
  return values
    .map((value) => (/\s/.test(value) ? JSON.stringify(value) : value))
    .join(" ");
}

const simpleScheduleKinds = new Set([
  "daily",
  "weekly",
  "monthly",
  "every_hours",
  "every_minutes",
  "every_seconds",
]);

export function renderTemplateValue(value, event, language) {
  const zh = language === "zh";
  const events = zh
    ? { start: "开始", success: "成功", failure: "失败" }
    : { start: "start", success: "success", failure: "failure" };
  const contents = zh
    ? {
        start: "备份已开始。",
        success: "备份已成功完成。",
        failure: "备份失败。原因：连接超时",
      }
    : {
        start: "Backup started.",
        success: "Backup completed successfully.",
        failure: "Backup failed. Reason: connection timed out",
      };
  const values = {
    plan_name: "Rclone Backup Test",
    event: events[event],
    content: contents[event],
    time: "2026-08-18 12:30:00 +00:00",
    backup_size:
      event === "success" ? "1.50 GiB" : zh ? "暂不可用" : "Not available",
  };
  return value.replace(
    /\{\{(plan_name|event|content|time|backup_size)\}\}/g,
    (_placeholder, key) => values[key],
  );
}

export function providerFieldIsSecret(option, name) {
  if (
    option.IsPassword ??
    option.isPassword ??
    option.Password ??
    option.password ??
    false
  )
    return true;
  const normalized = name.toLowerCase();
  const safeSensitiveIdentifiers = [
    "user",
    "username",
    "email",
    "access_key_id",
    "account_id",
    "client_id",
  ];
  const sensitive = option.Sensitive ?? option.sensitive ?? false;
  if (sensitive && !safeSensitiveIdentifiers.includes(normalized)) return true;
  return (
    normalized === "pass" ||
    normalized === "key" ||
    normalized === "headers" ||
    normalized.endsWith("_pass") ||
    normalized.includes("password") ||
    normalized.includes("secret") ||
    normalized.includes("token") ||
    normalized.includes("api_key") ||
    (normalized.endsWith("_key") && !normalized.endsWith("public_key")) ||
    normalized.includes("private_key") ||
    normalized.startsWith("private_") ||
    normalized.includes("credentials") ||
    [
      "access_grant",
      "authorization",
      "connection_string",
      "cookies",
      "key_pem",
      "master_key",
      "master_keys",
      "mnemonic",
      "sas_url",
    ].includes(normalized)
  );
}
