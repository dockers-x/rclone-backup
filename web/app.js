const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

const translations = {
  zh: {
    skip: "跳到主要内容", plans: "备份方案", history: "运行历史", serviceOnline: "服务在线",
    workspace: "工作空间", dashboard: "备份控制台", newPlan: "新建方案", overview: "运行概览",
    allBackups: "所有备份，一目了然", subtitle: "编排本地目录备份，通过 rclone 安全同步到任意远端。",
    schedulerActive: "调度器正在运行", schedulerWaiting: "调度器等待存储配置", activePlans: "启用方案",
    automatic: "自动调度", last24: "近 24 小时", successRuns: "次成功运行", successRate: "成功率",
    recentRuns: "最近运行", destinations: "远端目标", configuration: "配置", backupPlans: "备份方案",
    noPlans: "还没有备份方案", noPlansHint: "创建第一个方案，选择数据源、远端目标和重试策略。",
    createPlan: "创建方案", activity: "活动", recentHistory: "最近运行", refresh: "刷新",
    basics: "基础设置", basicsHint: "名称、运行状态与定时规则", planName: "方案名称", schedule: "Cron 表达式",
    scheduleHint: "例如：每天凌晨 2 点 = 0 2 * * *", timezone: "时区", enabled: "启用自动备份",
    enabledHint: "保存后调度器将按 Cron 规则运行", sourcesTargets: "数据源与目标",
    sourcesTargetsHint: "支持多个文件夹和 rclone 远端", folders: "备份文件夹", add: "添加",
    remoteTargets: "远端目标", rcloneFlags: "Rclone 全局参数", flagsHint: "使用 shell 风格引号解析，但不会通过 shell 执行",
    none: "不备份", archiveRetention: "归档、加密与保留", archiveRetentionHint: "生成可直接下载和解压恢复的标准归档",
    archiveType: "归档格式", archivePassword: "归档密码（可选）", fileSuffix: "文件名时间格式",
    secureArchive: "7z · 安全优先（推荐）", compatibleArchive: "ZIP · 兼容优先", nativeDirectory: "原生目录 · 依赖 rclone 恢复",
    secureArchiveHint: "设置后使用 AES-256 并加密文件名，常见 7z 软件可直接恢复。", compatibleArchiveHint: "设置后使用 ZipCrypto，兼容性广但加密较弱；敏感备份请选择 7z。", nativeDirectoryHint: "不生成归档，密码不生效；恢复时使用 rclone copy。",
    keepDays: "保留天数", keepCount: "保留份数", retryPolicy: "重试策略",
    retryHint: "在网络或远端临时故障时自动恢复", maxAttempts: "最大尝试次数", backoff: "退避方式",
    exponential: "指数退避", fixed: "固定间隔", initialDelay: "初始等待（秒）", maxDelay: "最长等待（秒）",
    notifications: "通知", notificationsHint: "Ping、SMTP 与 ServerChan", smtpHint: "在成功或失败时发送邮件",
    recipient: "收件人", smtpOptions: "s-nail 参数", serverChanHint: "通过 SendKey 推送运行状态",
    cancel: "取消", savePlan: "保存方案", runLog: "运行日志", label: "名称", path: "绝对路径",
    remoteName: "Rclone 远端名", remoteDir: "远端目录", edit: "编辑", runNow: "立即运行", delete: "删除",
    source: "数据源", target: "目标", retry: "重试", attempts: "次", enabledBadge: "已启用", disabledBadge: "已停用",
    manual: "手动", scheduleTrigger: "定时", cli: "命令行", noRuns: "暂无运行记录", viewLog: "查看日志",
    saveSuccess: "方案已保存", deleteConfirm: "确定删除这个方案吗？运行历史会保留。", deleteSuccess: "方案已删除",
    runQueued: "备份任务已加入队列", loadError: "加载失败", formInvalid: "请检查表单中的必填字段。",
    rcloneWaiting: "等待配置存储", rcloneWaitingHint: "服务会保持运行，但在检测到至少一个 rclone 远端前不会启动任何备份任务。",
    configureStorage: "配置存储", storageIntro: "选择存储提供商，输入 rclone 别名和该服务要求的凭据。敏感字段直接交给 rclone 加密保存。",
    rcloneAlias: "Rclone 别名", provider: "存储提供商", loadingProviders: "正在加载提供商…",
    advancedOptions: "高级选项", saveAndTest: "保存并测试", providerRequired: "请选择存储提供商。",
    remoteCreated: "存储配置已保存，正在测试连接…", remoteReady: "存储连接成功。调度器已解锁。",
    remoteNeedsInput: "rclone 需要更多信息，请完成下方步骤。", authenticationOff: "认证未启用",
    storageAccounts: "存储账号", addAccount: "添加账号", accountBoundary: "账号和密钥只保存在 rclone 配置文件中；备份数据库仅引用别名。", noAccounts: "还没有存储账号", noAccountsHint: "添加一个 rclone 远端，连接 S3、WebDAV、SFTP 或其他提供商。",
    test: "测试", testSuccess: "连接测试成功", remoteDeleteConfirm: "确定删除这个存储账号吗？", remoteDeleted: "存储账号已删除", remoteEditHint: "为保护已有凭据，编辑时只提交需要变更的字段。", remoteUpdate: "编辑账号",
  },
  en: {
    skip: "Skip to main content", plans: "Backup plans", history: "Run history", serviceOnline: "Service online",
    workspace: "Workspace", dashboard: "Backup console", newPlan: "New plan", overview: "Overview",
    allBackups: "Every backup, in view", subtitle: "Orchestrate local directory backups, then sync safely to any destination with rclone.",
    schedulerActive: "Scheduler is active", schedulerWaiting: "Scheduler is waiting for storage", activePlans: "Active plans",
    automatic: "Automatically scheduled", last24: "Last 24 hours", successRuns: "successful runs", successRate: "Success rate",
    recentRuns: "Recent runs", destinations: "Destinations", configuration: "Configuration", backupPlans: "Backup plans",
    noPlans: "No backup plans yet", noPlansHint: "Create your first plan and choose sources, destinations, and retry behavior.",
    createPlan: "Create plan", activity: "Activity", recentHistory: "Recent runs", refresh: "Refresh",
    basics: "Basics", basicsHint: "Name, status, and schedule", planName: "Plan name", schedule: "Cron expression",
    scheduleHint: "Example: daily at 02:00 = 0 2 * * *", timezone: "Timezone", enabled: "Enable automatic backup",
    enabledHint: "The scheduler will use this Cron rule after saving", sourcesTargets: "Sources & destinations",
    sourcesTargetsHint: "Multiple folders and rclone remotes are supported", folders: "Backup folders", add: "Add",
    remoteTargets: "Remote destinations", rcloneFlags: "Global rclone flags", flagsHint: "Parsed with shell-style quoting but never executed through a shell",
    none: "None", archiveRetention: "Archive, encryption & retention", archiveRetentionHint: "Create a standard archive that can be downloaded and extracted directly",
    archiveType: "Archive format", archivePassword: "Archive password (optional)", fileSuffix: "Filename time format",
    secureArchive: "7z · Security first (recommended)", compatibleArchive: "ZIP · Compatibility first", nativeDirectory: "Native directory · Restore with rclone",
    secureArchiveHint: "With a password, uses AES-256 and filename encryption. Common 7z apps can restore it directly.", compatibleArchiveHint: "With a password, uses widely compatible but weaker ZipCrypto. Choose 7z for sensitive backups.", nativeDirectoryHint: "No archive is created and this password is ignored. Restore with rclone copy.",
    keepDays: "Keep days", keepCount: "Keep count", retryPolicy: "Retry policy",
    retryHint: "Recover automatically from transient network or remote failures", maxAttempts: "Maximum attempts", backoff: "Backoff",
    exponential: "Exponential", fixed: "Fixed interval", initialDelay: "Initial delay (seconds)", maxDelay: "Maximum delay (seconds)",
    notifications: "Notifications", notificationsHint: "Ping, SMTP, and ServerChan", smtpHint: "Send mail on success or failure",
    recipient: "Recipient", smtpOptions: "s-nail options", serverChanHint: "Push run status with a SendKey",
    cancel: "Cancel", savePlan: "Save plan", runLog: "Run log", label: "Name", path: "Absolute path",
    remoteName: "Rclone remote", remoteDir: "Remote directory", edit: "Edit", runNow: "Run now", delete: "Delete",
    source: "Sources", target: "Targets", retry: "Retry", attempts: "attempts", enabledBadge: "Enabled", disabledBadge: "Disabled",
    manual: "Manual", scheduleTrigger: "Scheduled", cli: "CLI", noRuns: "No runs yet", viewLog: "View log",
    saveSuccess: "Plan saved", deleteConfirm: "Delete this plan? Run history will be kept.", deleteSuccess: "Plan deleted",
    runQueued: "Backup run queued", loadError: "Failed to load", formInvalid: "Check the required fields in the form.",
    rcloneWaiting: "Waiting for storage setup", rcloneWaitingHint: "The service stays online, but no backup can start until at least one rclone remote is detected.",
    configureStorage: "Configure storage", storageIntro: "Choose a provider, an rclone alias, and the credentials required by that service. Sensitive values go directly to rclone for encrypted storage.",
    rcloneAlias: "Rclone alias", provider: "Storage provider", loadingProviders: "Loading providers…",
    advancedOptions: "Advanced options", saveAndTest: "Save & test", providerRequired: "Choose a storage provider.",
    remoteCreated: "Storage configuration saved. Testing connection…", remoteReady: "Storage connected. The scheduler is now unlocked.",
    remoteNeedsInput: "rclone needs more information. Complete the next step below.", authenticationOff: "Authentication disabled",
    storageAccounts: "Storage accounts", addAccount: "Add account", accountBoundary: "Accounts and credentials live only in rclone.conf; the backup database stores alias references only.", noAccounts: "No storage accounts yet", noAccountsHint: "Add an rclone remote for S3, WebDAV, SFTP, or another provider.",
    test: "Test", testSuccess: "Connection test passed", remoteDeleteConfirm: "Delete this storage account?", remoteDeleted: "Storage account deleted", remoteEditHint: "To protect existing credentials, editing submits only fields you choose to change.", remoteUpdate: "Edit account",
  },
};

const state = {
  language: localStorage.getItem("language") || (navigator.language.startsWith("zh") ? "zh" : "en"),
  theme: localStorage.getItem("theme") || "system",
  plans: [], runs: [], remotes: [], status: null, editingId: null, providers: [], selectedProvider: null, remoteFlow: null, editingRemote: null,
};

function t(key) { return translations[state.language][key] || key; }
function icon(name) { return `<svg aria-hidden="true"><use href="#i-${name}"/></svg>`; }
function escapeHtml(value = "") {
  const node = document.createElement("div");
  node.textContent = String(value);
  return node.innerHTML;
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: { "Content-Type": "application/json", Accept: "application/json", ...options.headers },
  });
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try { message = (await response.json()).error || message; } catch {}
    throw new Error(message);
  }
  return response.status === 204 ? null : response.json();
}

function applyPreferences() {
  document.documentElement.dataset.theme = state.theme;
  document.documentElement.lang = state.language === "zh" ? "zh-CN" : "en";
  $("#languageButton span").textContent = state.language === "zh" ? "中文" : "EN";
  $("#newPlanButton").setAttribute("aria-label", t("newPlan"));
  $$("[data-i18n]").forEach((node) => { node.textContent = t(node.dataset.i18n); });
  if ($("#planDialog")?.open) updateArchiveHint();
}

function toast(message, error = false) {
  const item = document.createElement("div");
  item.className = `toast${error ? " error" : ""}`;
  item.textContent = message;
  $("#toastRegion").append(item);
  setTimeout(() => {
    item.dataset.leaving = "";
    setTimeout(() => item.remove(), 180);
  }, 3500);
}

async function loadAll() {
  try {
    const [plans, runs, status, health, remoteResponse] = await Promise.all([
      api("/api/plans"), api("/api/runs?limit=50"), api("/api/status"), api("/api/health"), api("/api/rclone/remotes"),
    ]);
    Object.assign(state, { plans, runs, status, remotes: remoteResponse.remotes || [] });
    $("#version").textContent = `v${health.version}`;
    render();
  } catch (error) {
    toast(`${t("loadError")}: ${error.message}`, true);
  }
}

function render() {
  applyPreferences();
  renderMetrics();
  renderPlans();
  renderAccounts();
  renderRuns();
  renderStatus();
}

function renderStatus() {
  const ready = Boolean(state.status?.rclone_ready);
  $("#readinessBanner").hidden = ready;
  $(".hero-status span:last-child").textContent = ready ? t("schedulerActive") : t("schedulerWaiting");
  $(".hero-status .pulse").style.background = ready ? "var(--success)" : "var(--accent)";
  $(".sidebar-foot > span:nth-child(2)").textContent = ready ? t("serviceOnline") : t("rcloneWaiting");
}

function renderMetrics() {
  const active = state.plans.filter((plan) => plan.enabled).length;
  const recent = state.runs.slice(0, 20);
  const successes = recent.filter((run) => run.status === "success").length;
  const complete = recent.filter((run) => ["success", "failed"].includes(run.status)).length;
  const last24 = state.runs.filter((run) => run.status === "success" && Date.now() - Date.parse(run.started_at) < 86400000).length;
  $("#activeCount").textContent = active;
  $("#successCount").textContent = last24;
  $("#successRate").textContent = complete ? `${Math.round(successes / complete * 100)}%` : "—";
  $("#remoteCount").textContent = state.remotes.length;
}

function renderAccounts() {
  const grid = $("#accountGrid");
  $("#emptyAccounts").hidden = state.remotes.length > 0;
  grid.hidden = state.remotes.length === 0;
  grid.innerHTML = state.remotes.map((remote) => `<article class="account-card" data-name="${escapeHtml(remote.name)}" data-type="${escapeHtml(remote.type)}">
    <div class="account-card-head"><div class="account-card-icon">${icon("globe")}</div><div><h3>${escapeHtml(remote.name)}</h3><small>${escapeHtml(remote.type)}</small></div></div>
    <div class="account-actions"><button class="button ghost" data-action="test-remote">${t("test")}</button><button class="icon-button" data-action="edit-remote" aria-label="${t("edit")}">${icon("edit")}</button><button class="icon-button danger" data-action="delete-remote" aria-label="${t("delete")}">${icon("trash")}</button></div>
  </article>`).join("");
}

function renderPlans() {
  const grid = $("#planGrid");
  $("#planCount").textContent = state.plans.length;
  $("#emptyPlans").hidden = state.plans.length > 0;
  grid.hidden = state.plans.length === 0;
  grid.innerHTML = state.plans.map((plan) => {
    const source = plan.sources.length === 1 ? plan.sources[0].name : `${plan.sources.length} ${t("folders")}`;
    const target = plan.remotes.length === 1 ? plan.remotes[0].name : `${plan.remotes.length} ${t("destinations")}`;
    return `<article class="plan-card" data-id="${plan.id}">
      <div class="plan-card-head"><div><p class="eyebrow">${escapeHtml(plan.timezone)}</p><h3>${escapeHtml(plan.name)}</h3><span class="mono">${escapeHtml(plan.schedule)}</span></div><span class="badge ${plan.enabled ? "enabled" : "disabled"}">${plan.enabled ? t("enabledBadge") : t("disabledBadge")}</span></div>
      <div class="plan-facts"><div class="fact"><span>${t("source")}</span><strong title="${escapeHtml(source)}">${escapeHtml(source)}</strong></div><div class="fact"><span>${t("target")}</span><strong title="${escapeHtml(target)}">${escapeHtml(target)}</strong></div><div class="fact"><span>${t("retry")}</span><strong>${plan.retry.max_attempts} ${t("attempts")}</strong></div></div>
      <div class="plan-actions"><button class="button primary" data-action="run" ${state.status?.rclone_ready ? "" : "disabled"}>${icon("play")}<span>${t("runNow")}</span></button><button class="icon-button" data-action="edit" aria-label="${t("edit")}">${icon("edit")}</button><button class="icon-button danger" data-action="delete" aria-label="${t("delete")}">${icon("trash")}</button></div>
    </article>`;
  }).join("");
}

function renderRuns() {
  const list = $("#historyList");
  if (!state.runs.length) {
    list.innerHTML = `<div class="empty-state"><p>${t("noRuns")}</p></div>`;
    return;
  }
  list.innerHTML = state.runs.map((run) => `<div class="history-row" data-run="${run.id}">
    <strong>${escapeHtml(run.plan_name)}</strong><time datetime="${run.started_at}">${new Intl.DateTimeFormat(state.language === "zh" ? "zh-CN" : "en", { dateStyle: "medium", timeStyle: "short" }).format(new Date(run.started_at))}</time>
    <span class="trigger">${t(run.trigger === "schedule" ? "scheduleTrigger" : run.trigger)}</span><span class="badge ${run.status}">${escapeHtml(run.status)}</span>
    <button class="icon-button" data-action="log" aria-label="${t("viewLog")}">${icon("chevron")}</button></div>`).join("");
}

function appendRow(kind, value = {}) {
  const template = $(`#${kind}Template`);
  const row = template.content.firstElementChild.cloneNode(true);
  $$("[data-field]", row).forEach((input) => {
    if (input.dataset.field === "name" && input.tagName === "SELECT") {
      input.innerHTML = `<option value="">—</option>` + state.remotes.map((remote) => `<option value="${escapeHtml(remote.name)}">${escapeHtml(remote.name)} · ${escapeHtml(remote.type)}</option>`).join("");
    }
    input.value = value[input.dataset.field] || "";
  });
  $(kind === "source" ? "#sourcesEditor" : "#remotesEditor").append(row);
  applyPreferences();
}

function openPlan(plan = null) {
  state.editingId = plan?.id || null;
  const form = $("#planForm");
  form.reset();
  $("#formError").hidden = true;
  $("#sourcesEditor").innerHTML = "";
  $("#remotesEditor").innerHTML = "";
  $("#dialogTitle").textContent = plan ? t("edit") : t("newPlan");
  const data = plan || {
    name: "", enabled: true, schedule: "5 * * * *", timezone: "UTC",
    sources: [{ name: "data", path: "/data" }], remotes: [{ name: "RcloneBackup", directory: "/RcloneBackup/" }],
    archive: { kind: "7z", password: "", suffix: "%Y%m%d-%H%M%S" },
    retention: { keep_days: 0, keep_count: 0 }, retry: { max_attempts: 3, initial_delay_seconds: 10, max_delay_seconds: 300, backoff: "exponential" },
    notifications: { ping: {}, mail: {}, serverchan: {} }, rclone_flags: [],
  };
  for (const [name, value] of Object.entries({
    name: data.name, schedule: data.schedule, timezone: data.timezone, enabled: data.enabled,
    rclone_flags: joinArgs(data.rclone_flags), archive_kind: data.archive.kind, archive_password: data.archive.password,
    archive_suffix: data.archive.suffix, keep_days: data.retention.keep_days, keep_count: data.retention.keep_count,
    max_attempts: data.retry.max_attempts, initial_delay: data.retry.initial_delay_seconds, max_delay: data.retry.max_delay_seconds,
    backoff: data.retry.backoff, ping_success: data.notifications.ping?.success_url || "",
    ping_failure: data.notifications.ping?.failure_url || "", mail_enabled: data.notifications.mail?.enabled || false,
    mail_to: data.notifications.mail?.to || "", mail_options: joinArgs(data.notifications.mail?.smtp_options || []),
    server_enabled: data.notifications.serverchan?.enabled || false, server_key: data.notifications.serverchan?.send_key || "",
  })) {
    const input = form.elements[name];
    if (!input) continue;
    if (input.type === "checkbox") input.checked = Boolean(value); else input.value = value ?? "";
  }
  data.sources.forEach((value) => appendRow("source", value));
  data.remotes.forEach((value) => appendRow("remote", value));
  updateArchiveHint();
  $("#planDialog").showModal();
  setTimeout(() => form.elements.name.focus(), 30);
}

function splitArgs(value) {
  const result = []; let current = ""; let quote = null; let escaped = false;
  for (const char of value.trim()) {
    if (escaped) { current += char; escaped = false; continue; }
    if (char === "\\") { escaped = true; continue; }
    if (quote) { if (char === quote) quote = null; else current += char; continue; }
    if (char === "'" || char === '"') { quote = char; continue; }
    if (/\s/.test(char)) { if (current) { result.push(current); current = ""; } } else current += char;
  }
  if (current) result.push(current);
  return result;
}
function joinArgs(values) { return values.map((value) => /\s/.test(value) ? JSON.stringify(value) : value).join(" "); }

function collectPlan() {
  const form = $("#planForm");
  if (!form.reportValidity()) throw new Error(t("formInvalid"));
  const value = (name) => form.elements[name]?.value?.trim() || "";
  const number = (name) => Number(form.elements[name]?.value || 0);
  return {
    name: value("name"), enabled: form.elements.enabled.checked, schedule: value("schedule"), timezone: value("timezone"),
    sources: $$(".source-row").map((row) => ({ name: $('[data-field="name"]', row).value.trim(), path: $('[data-field="path"]', row).value.trim() })),
    archive: { kind: value("archive_kind"), password: value("archive_password"), suffix: value("archive_suffix") },
    remotes: $$(".remote-row").map((row) => ({ name: $('[data-field="name"]', row).value.trim(), directory: $('[data-field="directory"]', row).value.trim() })),
    retention: { keep_days: number("keep_days"), keep_count: number("keep_count") },
    retry: { max_attempts: number("max_attempts"), initial_delay_seconds: number("initial_delay"), max_delay_seconds: number("max_delay"), backoff: value("backoff") },
    notifications: {
      ping: { completion_url: "", completion_options: [], start_url: "", start_options: [], success_url: value("ping_success"), success_options: [], failure_url: value("ping_failure"), failure_options: [] },
      mail: { enabled: form.elements.mail_enabled.checked, smtp_options: splitArgs(value("mail_options")), to: value("mail_to"), on_success: true, on_failure: true },
      serverchan: { enabled: form.elements.server_enabled.checked, send_key: value("server_key"), on_start: true, on_success: true, on_failure: true },
    },
    rclone_flags: splitArgs(value("rclone_flags")),
  };
}

async function savePlan(event) {
  event.preventDefault();
  const button = $("#saveButton");
  try {
    button.disabled = true;
    const body = collectPlan();
    await api(state.editingId ? `/api/plans/${state.editingId}` : "/api/plans", { method: state.editingId ? "PUT" : "POST", body: JSON.stringify(body) });
    $("#planDialog").close();
    toast(t("saveSuccess"));
    await loadAll();
  } catch (error) {
    $("#formError").textContent = error.message;
    $("#formError").hidden = false;
  } finally { button.disabled = false; }
}

async function openRemoteWizard(remote = null) {
  const dialog = $("#remoteDialog");
  $("#remoteError").hidden = true;
  $("#remoteResult").hidden = true;
  $("#remoteFlowFields").innerHTML = "";
  state.remoteFlow = null;
  state.editingRemote = remote;
  $("#remoteForm").reset();
  $("#remoteDialogTitle").textContent = remote ? t("remoteUpdate") : t("configureStorage");
  dialog.showModal();
  try {
    const response = await api("/api/rclone/providers");
    state.providers = response.providers || [];
    const select = $("#providerSelect");
    select.innerHTML = `<option value="">${t("provider")}</option>` + state.providers
      .slice().sort((a, b) => providerLabel(a).localeCompare(providerLabel(b)))
      .map((provider) => `<option value="${escapeHtml(provider.Name || provider.name || provider.Prefix || provider.prefix)}">${escapeHtml(providerLabel(provider))}</option>`).join("");
    if (remote) {
      $("#remoteForm").elements.remote_name.value = remote.name;
      $("#remoteForm").elements.remote_name.readOnly = true;
      select.value = remote.type;
      select.disabled = true;
      selectProvider();
      $("#remoteResult").textContent = t("remoteEditHint");
      $("#remoteResult").hidden = false;
    } else {
      $("#remoteForm").elements.remote_name.readOnly = false;
      select.disabled = false;
    }
  } catch (error) {
    $("#remoteError").textContent = error.message;
    $("#remoteError").hidden = false;
  }
}

function providerLabel(provider) {
  return provider.Description || provider.description || provider.Name || provider.name || provider.Prefix || provider.prefix || "Unknown";
}

function selectProvider() {
  const key = $("#providerSelect").value;
  state.selectedProvider = state.providers.find((provider) => [provider.Name, provider.name, provider.Prefix, provider.prefix].includes(key));
  const provider = state.selectedProvider;
  $("#providerFields").innerHTML = "";
  $("#advancedProviderFields").innerHTML = "";
  if (!provider) return;
  $("#providerDescription").textContent = provider.Description || provider.description || "";
  const options = provider.Options || provider.options || [];
  options.forEach((option) => {
    const advanced = option.Advanced ?? option.advanced ?? false;
    const target = advanced ? $("#advancedProviderFields") : $("#providerFields");
    target.append(providerField(option));
  });
  $("#advancedProvider").hidden = !$("#advancedProviderFields").children.length;
}

function providerField(option) {
  const name = option.Name || option.name;
  const label = option.Help || option.help || name;
  const required = !state.editingRemote && (option.Required ?? option.required ?? false);
  const password = option.IsPassword ?? option.isPassword ?? option.Password ?? option.password ?? false;
  const examples = option.Examples || option.examples || [];
  const defaultValue = state.editingRemote ? "" : (option.Default ?? option.default ?? "");
  const field = document.createElement("label");
  field.className = "field";
  const title = document.createElement("span");
  title.textContent = name + (required ? " *" : "");
  field.append(title);
  if (examples.length) {
    const select = document.createElement("select");
    select.name = `provider_${name}`;
    if (!required) select.append(new Option("—", ""));
    examples.forEach((example) => select.append(new Option(example.Help || example.help || String(example.Value ?? example.value), example.Value ?? example.value)));
    if (defaultValue !== null && defaultValue !== undefined) select.value = String(defaultValue);
    select.required = required;
    field.append(select);
  } else {
    const input = document.createElement("input");
    input.name = `provider_${name}`;
    input.value = defaultValue === null || defaultValue === undefined ? "" : String(defaultValue);
    input.required = required;
    input.type = password ? "password" : "text";
    if (password) input.autocomplete = "new-password";
    field.append(input);
  }
  if (label && label !== name) {
    const help = document.createElement("small");
    help.textContent = String(label).split("\n")[0];
    field.append(help);
  }
  return field;
}

function renderRemoteFlow(response) {
  const option = response.Option || response.option;
  if (!option) return;
  const field = providerField(option);
  const input = $("input, select", field);
  input.name = "remote_flow_result";
  input.required = true;
  $("#remoteFlowFields").replaceChildren(field);
  input.focus();
}

async function saveRemote(event) {
  event.preventDefault();
  const form = $("#remoteForm");
  if (!form.reportValidity() || !state.selectedProvider) {
    $("#remoteError").textContent = t("providerRequired");
    $("#remoteError").hidden = false;
    return;
  }
  const button = $("#remoteSaveButton");
  button.disabled = true;
  try {
    const parameters = {};
    $$('[name^="provider_"]', form).forEach((input) => {
      if (input.value !== "") parameters[input.name.slice(9)] = input.value;
    });
    const name = form.elements.remote_name.value.trim();
    const providerType = state.selectedProvider.Name || state.selectedProvider.name || state.selectedProvider.Prefix || state.selectedProvider.prefix;
    const payload = { name, type: providerType, parameters };
    if (state.remoteFlow) {
      const flowInput = form.elements.remote_flow_result;
      Object.assign(payload, state.remoteFlow, { result: flowInput ? flowInput.value : state.remoteFlow.result });
    }
    const result = state.editingRemote
      ? await api(`/api/rclone/remotes/${encodeURIComponent(name)}`, { method: "PUT", body: JSON.stringify({ parameters }) })
      : await api("/api/rclone/remotes", { method: "POST", body: JSON.stringify(payload) });
    if (result.State || result.state || result.Option || result.option) {
      state.remoteFlow = { state: result.State || result.state, result: result.Result || result.result || "" };
      renderRemoteFlow(result);
      $("#remoteResult").textContent = result.Option?.Help || result.option?.help || t("remoteNeedsInput");
      $("#remoteResult").hidden = false;
      toast(t("remoteNeedsInput"));
      return;
    }
    toast(t("remoteCreated"));
    if (!state.editingRemote) await api(`/api/rclone/remotes/${encodeURIComponent(name)}/test`, { method: "POST" });
    toast(t("remoteReady"));
    $("#remoteDialog").close();
    await loadAll();
  } catch (error) {
    $("#remoteError").textContent = error.message;
    $("#remoteError").hidden = false;
  } finally { button.disabled = false; }
}

document.addEventListener("click", async (event) => {
  const button = event.target.closest("[data-action], #newPlanButton, #configureRemoteButton, #addAccountButton, #themeButton, #languageButton, #menuButton, #refreshButton");
  if (!button) return;
  const action = button.dataset.action;
  if (button.id === "newPlanButton" || action === "new") openPlan();
  if (["configureRemoteButton", "addAccountButton"].includes(button.id) || action === "configure-storage") openRemoteWizard();
  if (button.id === "themeButton") {
    state.theme = ({ system: "light", light: "dark", dark: "system" })[state.theme];
    localStorage.setItem("theme", state.theme); applyPreferences();
  }
  if (button.id === "languageButton") {
    state.language = state.language === "zh" ? "en" : "zh";
    localStorage.setItem("language", state.language); render();
  }
  if (button.id === "menuButton") document.body.classList.toggle("menu-open");
  if (button.id === "refreshButton") loadAll();
  if (["close", "close-log", "close-remote"].includes(action)) button.closest("dialog").close();
  if (action === "add-source") appendRow("source");
  if (action === "add-remote") appendRow("remote");
  if (action === "remove-row") button.closest(".repeat-row").remove();
  const card = button.closest(".plan-card");
  if (card && action === "edit") openPlan(state.plans.find((plan) => plan.id === card.dataset.id));
  if (card && action === "run") {
    button.disabled = true;
    try { await api(`/api/plans/${card.dataset.id}/run`, { method: "POST" }); toast(t("runQueued")); await loadAll(); }
    catch (error) { toast(error.message, true); } finally { button.disabled = false; }
  }
  if (card && action === "delete" && confirm(t("deleteConfirm"))) {
    try { await api(`/api/plans/${card.dataset.id}`, { method: "DELETE" }); toast(t("deleteSuccess")); await loadAll(); }
    catch (error) { toast(error.message, true); }
  }
  if (action === "log") {
    const run = state.runs.find((item) => item.id === button.closest(".history-row").dataset.run);
    $("#logContent").textContent = run?.log || t("noRuns"); $("#logDialog").showModal();
  }
  const account = button.closest(".account-card");
  if (account && action === "test-remote") {
    button.disabled = true;
    try { await api(`/api/rclone/remotes/${encodeURIComponent(account.dataset.name)}/test`, { method: "POST" }); toast(t("testSuccess")); }
    catch (error) { toast(error.message, true); } finally { button.disabled = false; }
  }
  if (account && action === "edit-remote") openRemoteWizard({ name: account.dataset.name, type: account.dataset.type });
  if (account && action === "delete-remote" && confirm(t("remoteDeleteConfirm"))) {
    try { await api(`/api/rclone/remotes/${encodeURIComponent(account.dataset.name)}`, { method: "DELETE" }); toast(t("remoteDeleted")); await loadAll(); }
    catch (error) { toast(error.message, true); }
  }
});

function updateArchiveHint() {
  const form = $("#planForm");
  const kind = form.elements.archive_kind.value;
  $("#archiveSecurityHint").textContent = t(kind === "7z" ? "secureArchiveHint" : kind === "zip" ? "compatibleArchiveHint" : "nativeDirectoryHint");
  form.elements.archive_password.disabled = kind === "none";
}

document.addEventListener("click", (event) => {
  if (document.body.classList.contains("menu-open") && !event.target.closest(".sidebar, #menuButton")) document.body.classList.remove("menu-open");
});
$("#planForm").addEventListener("submit", savePlan);
$("#remoteForm").addEventListener("submit", saveRemote);
$("#providerSelect").addEventListener("change", selectProvider);
$("#planForm").elements.archive_kind.addEventListener("change", updateArchiveHint);
$$("dialog").forEach((dialog) => dialog.addEventListener("click", (event) => {
  const rect = dialog.getBoundingClientRect();
  if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) dialog.close();
}));

applyPreferences();
loadAll();
setInterval(async () => {
  if (document.visibilityState === "visible") {
    try {
      state.status = await api("/api/status");
      state.runs = await api("/api/runs?limit=50");
      renderStatus(); renderMetrics(); renderRuns();
    } catch {}
  }
}, 5000);
