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
    basics: "基础设置", basicsHint: "名称、运行状态与定时规则", planName: "方案名称", schedule: "Cron 表达式", scheduleMode: "定时方式", simpleSchedule: "简单定时",
    scheduleHint: "支持 5、6 或 7 段 Cron，例如：0 2 * * *", timezone: "时区", enabled: "启用自动备份",
    enabledHint: "保存后调度器将按当前定时规则运行", scheduleFrequency: "运行频率", daily: "每天", weekly: "每周", monthly: "每月", everyHours: "每隔几小时", everyMinutes: "每隔几分钟", everySeconds: "每隔几秒", runAt: "运行时间", weekday: "星期", monthday: "每月日期", monthdayHint: "没有该日期的月份会跳过", interval: "间隔", hoursUnit: "小时", minutesUnit: "分钟", secondsUnit: "秒", monday: "星期一", tuesday: "星期二", wednesday: "星期三", thursday: "星期四", friday: "星期五", saturday: "星期六", sunday: "星期日", schedulePreview: "{summary}",
    sourcesTargets: "数据源与目标",
    sourcesTargetsHint: "支持多个文件夹和 rclone 远端", folders: "备份文件夹", add: "添加",
    remoteTargets: "远端目标", remoteCheckConcurrency: "连接检查并行数", remoteCheckConcurrencyHint: "同时检查或创建远端目录的最大数量", rcloneFlags: "Rclone 全局参数", flagsHint: "使用 shell 风格引号解析，但不会通过 shell 执行",
    none: "不备份", archiveEncryption: "归档与加密", archiveEncryptionHint: "生成可直接下载和解压恢复的标准归档", backupRetentionPolicy: "备份保留策略", backupRetentionHint: "按时间或数量自动清理旧备份，可独立启用",
    archiveType: "归档格式", archivePassword: "归档密码（可选）", fileSuffix: "文件名时间格式",
    secureArchive: "7z · 安全优先（推荐）", compatibleArchive: "ZIP · 兼容优先", nativeDirectory: "原生目录 · 依赖 rclone 恢复",
    secureArchiveHint: "设置后使用 AES-256 并加密文件名，常见 7z 软件可直接恢复。", compatibleArchiveHint: "设置后使用 ZipCrypto，兼容性广但加密较弱；敏感备份请选择 7z。", nativeDirectoryHint: "不生成归档，密码不生效；恢复时使用 rclone copy。",
    keepDays: "保留天数", keepDaysHint: "删除超过指定天数的备份", keepCount: "保留份数", keepCountHint: "仅保留最新的指定份数", daysUnit: "天", copiesUnit: "份", passwordAlreadySet: "密码已设置。可点击眼睛查看，留空保持不变。", passwordRevealNeedsAuth: "启用 Web 密码认证后才能查看已有归档密码。", passwordHint: "密码提示（可选）", passwordHintHelp: "用于帮助回忆，请勿填写密码本身", retryPolicy: "重试策略",
    retryHint: "在网络或远端临时故障时自动恢复", maxAttempts: "失败重试次数", retryCountHint: "不包含首次执行；填 0 表示失败后不重试", backoff: "退避方式",
    exponential: "指数退避", fixed: "固定间隔", initialDelay: "初始等待（秒）", maxDelay: "最长等待（秒）",
    notifications: "通知", notificationsHint: "Ping、SMTP 与 ServerChan", smtpHint: "在成功或失败时发送邮件",
    recipient: "收件人", smtpOptions: "SMTP 参数", serverChanHint: "通过 SendKey 推送运行状态",
    cancel: "取消", savePlan: "保存方案", runLog: "运行日志", runProgress: "运行进度", technicalLog: "技术日志", label: "名称", path: "绝对路径",
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
    runStatus: "运行状态", phase: "当前阶段", attemptLabel: "第 {current}/{total} 次尝试", targetProgress: "存储目标", showPassword: "显示密码", hidePassword: "隐藏密码",
    pending: "等待中", checking: "检查连接", ready: "连接可用", uploading: "上传中", success: "成功", failed: "失败", unavailable: "不可用", running: "运行中", retrying: "等待重试",
    checking_destinations: "检查存储目标", preparing_files: "准备文件", creating_archive: "创建归档", retention: "执行保留策略", completed: "备份完成",
    globalSettings: "全局设置", notConfigured: "未配置", configured: "已启用", globalNotificationHint: "所有手动与定时备份共用这一套通知配置。",
    pingHint: "向健康检查或 Webhook URL 报告备份事件", enablePing: "启用 Ping", completionUrl: "完成 URL（成功或失败）", startUrl: "开始 URL", successUrl: "成功 URL", failureUrl: "失败 URL",
    onStart: "开始", onSuccess: "成功", onFailure: "失败", smtpGlobalHint: "需填写 smtps:// 服务器和发件地址", enableSmtp: "启用 SMTP", enableServerChan: "启用 ServerChan",
    sendTest: "发送测试", testingNotification: "正在发送…", notificationTestSuccess: "测试成功", saveNotifications: "保存通知配置", notificationSaved: "通知配置已保存", updatedAt: "更新于", addNotification: "添加目标", noNotificationTargets: "还没有通知目标", noNotificationTargetsHint: "添加 Ping、Email、Server酱或 ntfy；同一类型可以添加多个。", notificationName: "目标名称", notificationEvents: "通知事件", notificationTargets: "通知目标", notificationType: "通知类型", enableNotification: "启用通知目标", serverChanApp: "Server酱 App 推送", serverChanWechat: "Server酱微信推送", removeNotification: "删除目标", collapse: "收起", testFailed: "测试失败",
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
    basics: "Basics", basicsHint: "Name, status, and schedule", planName: "Plan name", schedule: "Cron expression", scheduleMode: "Schedule mode", simpleSchedule: "Simple schedule",
    scheduleHint: "Supports 5, 6, or 7 Cron fields, for example: 0 2 * * *", timezone: "Timezone", enabled: "Enable automatic backup",
    enabledHint: "The scheduler will use this schedule after saving", scheduleFrequency: "Frequency", daily: "Daily", weekly: "Weekly", monthly: "Monthly", everyHours: "Every few hours", everyMinutes: "Every few minutes", everySeconds: "Every few seconds", runAt: "Run at", weekday: "Weekday", monthday: "Day of month", monthdayHint: "Months without this date are skipped", interval: "Interval", hoursUnit: "hours", minutesUnit: "minutes", secondsUnit: "seconds", monday: "Monday", tuesday: "Tuesday", wednesday: "Wednesday", thursday: "Thursday", friday: "Friday", saturday: "Saturday", sunday: "Sunday", schedulePreview: "{summary}",
    sourcesTargets: "Sources & destinations",
    sourcesTargetsHint: "Multiple folders and rclone remotes are supported", folders: "Backup folders", add: "Add",
    remoteTargets: "Remote destinations", remoteCheckConcurrency: "Concurrent destination checks", remoteCheckConcurrencyHint: "Maximum destinations checked or created at the same time", rcloneFlags: "Global rclone flags", flagsHint: "Parsed with shell-style quoting but never executed through a shell",
    none: "None", archiveEncryption: "Archive & encryption", archiveEncryptionHint: "Create a standard archive that can be downloaded and extracted directly", backupRetentionPolicy: "Backup retention policy", backupRetentionHint: "Clean up old backups by age or count; enable either independently",
    archiveType: "Archive format", archivePassword: "Archive password (optional)", fileSuffix: "Filename time format",
    secureArchive: "7z · Security first (recommended)", compatibleArchive: "ZIP · Compatibility first", nativeDirectory: "Native directory · Restore with rclone",
    secureArchiveHint: "With a password, uses AES-256 and filename encryption. Common 7z apps can restore it directly.", compatibleArchiveHint: "With a password, uses widely compatible but weaker ZipCrypto. Choose 7z for sensitive backups.", nativeDirectoryHint: "No archive is created and this password is ignored. Restore with rclone copy.",
    keepDays: "Keep by age", keepDaysHint: "Delete backups older than the specified number of days", keepCount: "Keep by count", keepCountHint: "Keep only the specified number of newest backups", daysUnit: "days", copiesUnit: "copies", passwordAlreadySet: "A password is set. Use the eye to reveal it, or leave blank to keep it.", passwordRevealNeedsAuth: "Enable web password authentication to reveal an existing archive password.", passwordHint: "Password hint (optional)", passwordHintHelp: "A memory aid only; do not enter the password itself", retryPolicy: "Retry policy",
    retryHint: "Recover automatically from transient network or remote failures", maxAttempts: "Retries after failure", retryCountHint: "Excludes the first attempt; use 0 to disable retries", backoff: "Backoff",
    exponential: "Exponential", fixed: "Fixed interval", initialDelay: "Initial delay (seconds)", maxDelay: "Maximum delay (seconds)",
    notifications: "Notifications", notificationsHint: "Ping, SMTP, and ServerChan", smtpHint: "Send mail on success or failure",
    recipient: "Recipient", smtpOptions: "SMTP options", serverChanHint: "Push run status with a SendKey",
    cancel: "Cancel", savePlan: "Save plan", runLog: "Run log", runProgress: "Run progress", technicalLog: "Technical log", label: "Name", path: "Absolute path",
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
    runStatus: "Run status", phase: "Current stage", attemptLabel: "Attempt {current}/{total}", targetProgress: "Storage targets", showPassword: "Show password", hidePassword: "Hide password",
    pending: "Pending", checking: "Checking", ready: "Ready", uploading: "Uploading", success: "Succeeded", failed: "Failed", unavailable: "Unavailable", running: "Running", retrying: "Retrying",
    checking_destinations: "Checking destinations", preparing_files: "Preparing files", creating_archive: "Creating archive", retention: "Applying retention", completed: "Backup completed",
    globalSettings: "Global settings", notConfigured: "Not configured", configured: "Enabled", globalNotificationHint: "All manual and scheduled backups share this notification configuration.",
    pingHint: "Report backup events to health-check or webhook URLs", enablePing: "Enable Ping", completionUrl: "Completion URL (success or failure)", startUrl: "Start URL", successUrl: "Success URL", failureUrl: "Failure URL",
    onStart: "Start", onSuccess: "Success", onFailure: "Failure", smtpGlobalHint: "Requires an smtps:// server and sender address", enableSmtp: "Enable SMTP", enableServerChan: "Enable ServerChan",
    sendTest: "Send test", testingNotification: "Sending…", notificationTestSuccess: "Test succeeded", saveNotifications: "Save notifications", notificationSaved: "Notification settings saved", updatedAt: "Updated", addNotification: "Add target", noNotificationTargets: "No notification targets", noNotificationTargetsHint: "Add Ping, Email, ServerChan, or ntfy. You can add more than one of each type.", notificationName: "Target name", notificationEvents: "Notification events", notificationTargets: "Notification targets", notificationType: "Notification type", enableNotification: "Enable notification target", serverChanApp: "ServerChan App Push", serverChanWechat: "ServerChan WeChat Push", removeNotification: "Delete target", collapse: "Collapse", testFailed: "Test failed",
  },
};

const state = {
  language: localStorage.getItem("language") || (navigator.language.startsWith("zh") ? "zh" : "en"),
  theme: localStorage.getItem("theme") || "system",
  plans: [], runs: [], remotes: [], notifications: null, notificationTargets: [], expandedNotificationId: null, status: null, editingId: null, providers: [], selectedProvider: null, remoteFlow: null, editingRemote: null, openRunId: null, page: "plans",
};

const pageRoutes = {
  plans: { path: "/plans", title: "backupPlans" },
  accounts: { path: "/accounts", title: "storageAccounts" },
  notifications: { path: "/notifications", title: "notifications" },
  history: { path: "/history", title: "history" },
};
const pageNames = new Set(Object.keys(pageRoutes));
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
let navigationPointer = null;
let pageAnimation = null;

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
  $$("[data-feedback-key]").forEach((node) => { node.textContent = t(node.dataset.feedbackKey); });
  const passwordToggle = $("[data-action=\"toggle-password\"]");
  if (passwordToggle) passwordToggle.setAttribute("aria-label", t(passwordToggle.getAttribute("aria-pressed") === "true" ? "hidePassword" : "showPassword"));
  if ($("#planDialog")?.open) { updateArchiveHint(); updateScheduleBuilder(); }
  renderPage(state.page);
}

function pageFromPath(pathname = location.pathname) {
  const normalized = pathname !== "/" ? pathname.replace(/\/+$/, "") : pathname;
  return Object.entries(pageRoutes).find(([, route]) => route.path === normalized)?.[0] || "plans";
}

function legacyHashPage() {
  const page = location.hash.replace(/^#/, "");
  return pageNames.has(page) ? page : null;
}

function renderPage(page, { animate = false, focusHeading = false, resetScroll = false } = {}) {
  const nextPage = pageNames.has(page) ? page : "plans";
  state.page = nextPage;
  $$("[data-page]").forEach((view) => {
    view.getAnimations().forEach((animation) => animation.cancel());
    view.hidden = view.dataset.page !== nextPage;
  });
  $$("[data-page-link]").forEach((item) => {
    const active = item.dataset.pageLink === nextPage;
    item.classList.toggle("active", active);
    if (active) item.setAttribute("aria-current", "page"); else item.removeAttribute("aria-current");
  });
  $("#pageHeading").textContent = t(pageRoutes[nextPage].title);
  $("#newPlanButton").hidden = nextPage !== "plans";
  const view = $(`[data-page="${nextPage}"]`);
  if (resetScroll) window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  if (animate && !reduceMotion.matches && view) {
    pageAnimation = view.animate(
      [{ opacity: 0, transform: "translateX(8px)" }, { opacity: 1, transform: "translateX(0)" }],
      { duration: 170, easing: "cubic-bezier(.23, 1, .32, 1)" },
    );
  } else {
    pageAnimation = null;
  }
  if (focusHeading) requestAnimationFrame(() => $("#pageHeading").focus({ preventScroll: true }));
}

function navigateToPage(page, { push = false, animate = false, focusHeading = false, resetScroll = false } = {}) {
  const nextPage = pageNames.has(page) ? page : "plans";
  if (push && (state.page !== nextPage || location.pathname !== pageRoutes[nextPage].path)) {
    history.pushState({ page: nextPage }, "", `${pageRoutes[nextPage].path}${location.search}`);
  }
  renderPage(nextPage, { animate, focusHeading, resetScroll });
  setMenuOpen(false);
}

function initializeNavigation() {
  const hashPage = legacyHashPage();
  const page = hashPage || pageFromPath();
  if (hashPage) history.replaceState({ page }, "", `${pageRoutes[page].path}${location.search}`);
  else history.replaceState({ page }, "", `${location.pathname}${location.search}`);
  state.page = page;
}

function setMenuOpen(open) {
  document.body.classList.toggle("menu-open", open);
  $("#menuButton").setAttribute("aria-expanded", String(open));
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
    const [plans, runs, status, health, remoteResponse, notifications] = await Promise.all([
      api("/api/plans"), api("/api/runs?limit=50"), api("/api/status"), api("/api/health"), api("/api/rclone/remotes"), api("/api/notifications"),
    ]);
    Object.assign(state, { plans, runs, status, notifications, remotes: remoteResponse.remotes || [] });
    state.notificationTargets = structuredClone(notifications.config?.targets || []);
    const rcloneVersion = health.rclone_version ? ` · rclone ${health.rclone_version}` : "";
    $("#version").textContent = `v${health.version}${rcloneVersion}`;
    document.title = health.site_name;
    $("#siteName").textContent = health.site_name;
    $("#siteName").title = health.site_name;
    render(true);
  } catch (error) {
    toast(`${t("loadError")}: ${error.message}`, true);
  }
}

function render(syncNotifications = false) {
  applyPreferences();
  renderMetrics();
  renderPlans();
  renderAccounts();
  renderNotifications(syncNotifications);
  renderRuns();
  renderStatus();
}

function renderNotifications(syncFromServer = false) {
  const settings = state.notifications;
  if (!settings) return;
  if (syncFromServer) state.notificationTargets = structuredClone(settings.config?.targets || []);
  const active = settings.confirmed && state.notificationTargets.some((target) => target.enabled);
  const badge = $("#notificationState"); badge.textContent = t(active ? "configured" : "notConfigured"); badge.className = `badge ${active ? "enabled" : "disabled"}`;
  $("#notificationSavedAt").textContent = settings.confirmed ? `${t("updatedAt")} ${new Intl.DateTimeFormat(state.language === "zh" ? "zh-CN" : "en", { dateStyle: "medium", timeStyle: "short" }).format(new Date(settings.updated_at))}` : "";
  renderNotificationTargets();
}

function notificationTypeLabel(target) {
  if (target.type === "serverchan") return t(target.config?.channel === "app" ? "serverChanApp" : "serverChanWechat");
  return target.type === "email" ? "Email" : target.type === "ntfy" ? "ntfy" : "Ping";
}

function notificationDetail(target) {
  const config = target.config || {};
  if (target.type === "email") return config.to || t("notConfigured");
  if (target.type === "ntfy") return config.topic ? `${config.server || "ntfy"} / ${config.topic}` : t("notConfigured");
  if (target.type === "serverchan") return config.send_key ? "••••••••" : t("notConfigured");
  return config.completion_url || config.success_url || config.start_url || config.failure_url || t("notConfigured");
}

function notificationFields(target) {
  const config = target.config || {};
  const input = (label, name, value = "", type = "text", extra = "") => `<label class="field"><span>${escapeHtml(label)}</span><input data-notification-field="${name}" type="${type}" value="${escapeHtml(value)}" ${extra}></label>`;
  if (target.type === "email") return `${input(t("recipient"), "to", config.to, "email", "required autocomplete=\"email\"")}${input(t("smtpOptions"), "smtp_options", joinArgs(config.smtp_options || []), "text", "required autocomplete=\"off\"")}`;
  if (target.type === "serverchan") {
    const href = config.channel === "app" ? "https://sc3.ft07.com/sendkey" : "https://sct.ftqq.com/sendkey";
    return `${input("SendKey", "send_key", config.send_key, "password", "required autocomplete=\"new-password\"")}<a class="field-link" href="${href}" target="_blank" rel="noreferrer">${escapeHtml(notificationTypeLabel(target))} · SendKey</a>`;
  }
  if (target.type === "ntfy") return `${input("ntfy Server", "server", config.server || "https://ntfy.sh", "url", "required")}${input("Topic", "topic", config.topic, "text", "required pattern=\"[A-Za-z0-9_-]+\"")}${input("Token", "token", config.token, "password", "autocomplete=\"new-password\"")}`;
  return `${input(t("completionUrl"), "completion_url", config.completion_url, "password")}${input(t("startUrl"), "start_url", config.start_url, "password")}${input(t("successUrl"), "success_url", config.success_url, "password")}${input(t("failureUrl"), "failure_url", config.failure_url, "password")}`;
}

function renderNotificationTargets() {
  const list = $("#notificationList");
  list.setAttribute("aria-label", t("notificationTargets"));
  $("#notificationCount").textContent = `${state.notificationTargets.length} / 32`;
  $("#emptyNotifications").hidden = state.notificationTargets.length > 0;
  list.innerHTML = state.notificationTargets.map((target, index) => {
    const expanded = state.expandedNotificationId === target.id;
    const events = [target.on_start && t("onStart"), target.on_success && t("onSuccess"), target.on_failure && t("onFailure")].filter(Boolean).join(" · ") || "—";
    return `<li class="notification-target" data-id="${escapeHtml(target.id)}" style="--index:${index}">
      <div class="notification-row"><span class="channel-icon">${icon(target.type === "ping" ? "globe" : "bell")}</span><button type="button" class="notification-identity" data-action="toggle-notification" aria-expanded="${expanded}" aria-controls="notification-editor-${escapeHtml(target.id)}"><b>${escapeHtml(target.name)}</b><small>${escapeHtml(notificationDetail(target))}</small></button><span class="notification-meta"><span>${escapeHtml(notificationTypeLabel(target))}</span><small>${escapeHtml(events)}</small></span><label class="compact-switch"><input type="checkbox" data-notification-enabled ${target.enabled ? "checked" : ""} aria-label="${escapeHtml(`${t("enableNotification")} ${target.name}`)}"><i></i></label><button class="icon-button" type="button" data-action="toggle-notification" tabindex="-1" aria-label="${escapeHtml(t(expanded ? "collapse" : "edit"))}">${icon(expanded ? "chevron-up" : "chevron-down")}</button></div>
      <div class="notification-editor" id="notification-editor-${escapeHtml(target.id)}" ${expanded ? "" : "hidden"}><fieldset><legend>${escapeHtml(notificationTypeLabel(target))}</legend><div class="form-grid">${inputName(target)}${notificationFields(target)}</div><fieldset class="event-fieldset"><legend>${escapeHtml(t("notificationEvents"))}</legend><div class="event-options">${["start", "success", "failure"].map((event) => `<label><input type="checkbox" data-notification-event="${event}" ${target[`on_${event}`] ? "checked" : ""}><span>${escapeHtml(t(`on${event[0].toUpperCase()}${event.slice(1)}`))}</span></label>`).join("")}</div></fieldset><div class="notification-editor-actions"><button class="button ghost danger-text" type="button" data-action="remove-notification">${escapeHtml(t("removeNotification"))}</button><span class="target-test-status" role="status"></span><button class="button ghost" type="button" data-action="test-notification" data-id="${escapeHtml(target.id)}">${escapeHtml(t("sendTest"))}</button></div></fieldset></div>
    </li>`;
  }).join("");
}

function inputName(target) {
  return `<label class="field span-2"><span>${escapeHtml(t("notificationName"))}</span><input data-notification-name value="${escapeHtml(target.name)}" maxlength="80" required></label>`;
}

function collectNotifications() {
  if (!$("#notificationForm").reportValidity()) throw new Error(t("formInvalid"));
  return { targets: structuredClone(state.notificationTargets), ping: {}, mail: {}, serverchan: {} };
}

async function saveNotifications(event) {
  event.preventDefault(); const button = $("#saveNotificationsButton");
  try { button.disabled = true; const response = await api("/api/notifications", { method: "PUT", body: JSON.stringify({ config: collectNotifications() }) }); state.notifications = response; state.notificationTargets = structuredClone(response.config?.targets || []); $("#notificationError").hidden = true; renderNotifications(); toast(t("notificationSaved")); }
  catch (error) { $("#notificationError").textContent = error.message; $("#notificationError").hidden = false; }
  finally { button.disabled = false; }
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
      <div class="plan-facts"><div class="fact"><span>${t("source")}</span><strong title="${escapeHtml(source)}">${escapeHtml(source)}</strong></div><div class="fact"><span>${t("target")}</span><strong title="${escapeHtml(target)}">${escapeHtml(target)}</strong></div><div class="fact"><span>${t("retry")}</span><strong>${Math.max(0, plan.retry.max_attempts - 1)} ${t("attempts")}</strong></div></div>
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
    <span class="trigger">${t(run.trigger === "schedule" ? "scheduleTrigger" : run.trigger)}</span><span class="badge ${run.status}">${escapeHtml(t(run.status))}</span>
    <button class="icon-button" data-action="log" aria-label="${t("viewLog")}">${icon("chevron")}</button></div>`).join("");
}

function parseRunProgress(run) {
  const progress = { phase: run.status, targets: new Map(), log: [] };
  for (const line of (run.log || "").split("\n")) {
    const marker = line.indexOf("@event ");
    if (marker < 0) { if (line) progress.log.push(line); continue; }
    try {
      const event = JSON.parse(line.slice(marker + 7));
      if (event.kind === "phase") progress.phase = event.phase;
      if (event.kind === "target") progress.targets.set(`${event.name}\u0000${event.directory || ""}`, event);
    } catch { progress.log.push(line); }
  }
  return progress;
}

function renderOpenRun() {
  if (!state.openRunId || !$("#logDialog").open) return;
  const run = state.runs.find((item) => item.id === state.openRunId);
  if (!run) return;
  const progress = parseRunProgress(run);
  const plan = state.plans.find((item) => item.id === run.plan_id);
  const maxAttempts = plan?.retry?.max_attempts || Math.max(1, run.attempt);
  const status = run.status === "success" ? "success" : run.status === "failed" ? "failed" : "running";
  const targets = [...progress.targets.values()];
  const succeeded = targets.filter((item) => item.status === "success").length;
  const overview = $("#runOverview");
  const signature = JSON.stringify({ status, runStatus: run.status, attempt: run.attempt, maxAttempts, phase: progress.phase, targets });
  if (overview.dataset.signature !== signature) overview.innerHTML = `<div class="run-summary ${status}"><span class="run-state-icon">${icon(status === "success" ? "check" : status === "failed" ? "x" : "clock")}</span><div><small>${escapeHtml(t("runStatus"))}</small><strong>${escapeHtml(t(run.status))}</strong><span>${escapeHtml(t("attemptLabel").replace("{current}", run.attempt).replace("{total}", maxAttempts))}</span></div><div class="run-phase" role="status"><small>${escapeHtml(t("phase"))}</small><strong>${escapeHtml(t(progress.phase))}</strong></div></div>
    <div class="target-progress"><h3>${escapeHtml(t("targetProgress"))}<span>${succeeded}/${targets.length}</span></h3>${targets.length ? targets.map((target) => `<article class="target-status ${escapeHtml(target.status)}"><span class="target-icon">${icon(target.status === "success" || target.status === "ready" ? "check" : target.status === "failed" || target.status === "unavailable" ? "x" : "clock")}</span><div><strong>${escapeHtml(target.name)}</strong><small>${escapeHtml(target.directory || "")}${target.detail ? ` · ${escapeHtml(target.detail)}` : ""}</small></div><b>${escapeHtml(t(target.status))}</b></article>`).join("") : `<p class="target-empty">${escapeHtml(t("checking_destinations"))}</p>`}</div>`;
  overview.dataset.signature = signature;
  const pre = $("#logContent");
  pre.textContent = progress.log.join("\n") || t("noRuns");
  if (["running", "retrying"].includes(run.status)) pre.scrollTop = pre.scrollHeight;
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
  const passwordInput = form.elements.archive_password;
  const passwordToggle = $("[data-action=\"toggle-password\"]", form);
  passwordInput.type = "password";
  passwordInput.dataset.passwordSet = String(plan?.archive.password === "••••••••");
  passwordToggle.setAttribute("aria-pressed", "false");
  passwordToggle.setAttribute("aria-label", t("showPassword"));
  passwordToggle.innerHTML = icon("eye");
  $("#formError").hidden = true;
  $("#sourcesEditor").innerHTML = "";
  $("#remotesEditor").innerHTML = "";
  $("#dialogTitle").textContent = plan ? `${t("edit")} · ${plan.name}` : t("newPlan");
  const data = plan || {
    name: "", enabled: true, schedule: "0 0 2 * * *", timezone: "UTC",
    sources: [{ name: "data", path: "/data" }], remotes: [{ name: "RcloneBackup", directory: "/RcloneBackup/" }],
    archive: { kind: "7z", password: "", password_hint: "", suffix: "%Y%m%d-%H%M%S" },
    retention: { keep_days: 0, keep_count: 0 }, retry: { max_attempts: 3, initial_delay_seconds: 10, max_delay_seconds: 300, backoff: "exponential" },
    notifications: { ping: {}, mail: {}, serverchan: {} }, rclone_flags: [], remote_check_concurrency: 4,
  };
  for (const [name, value] of Object.entries({
    name: data.name, schedule: data.schedule, timezone: data.timezone, enabled: data.enabled,
    rclone_flags: joinArgs(data.rclone_flags), archive_kind: data.archive.kind, archive_password: data.archive.password === "••••••••" ? "" : data.archive.password,
    archive_suffix: data.archive.suffix, archive_password_hint: data.archive.password_hint || "", keep_days: data.retention.keep_days || 30, keep_count: data.retention.keep_count || 10,
    keep_days_enabled: data.retention.keep_days > 0, keep_count_enabled: data.retention.keep_count > 0,
    retry_count: Math.max(0, data.retry.max_attempts - 1), initial_delay: data.retry.initial_delay_seconds, max_delay: data.retry.max_delay_seconds,
    backoff: data.retry.backoff, remote_check_concurrency: data.remote_check_concurrency ?? 4,
  })) {
    const input = form.elements[name];
    if (!input) continue;
    if (input.type === "checkbox") input.checked = Boolean(value); else input.value = value ?? "";
  }
  const simpleSchedule = parseSimpleSchedule(data.schedule);
  form.elements.schedule_mode.value = simpleSchedule ? "simple" : "cron";
  if (simpleSchedule) {
    form.elements.schedule_kind.value = simpleSchedule.kind;
    if (simpleSchedule.time) form.elements.schedule_time.value = simpleSchedule.time;
    if (simpleSchedule.weekday) form.elements.schedule_weekday.value = simpleSchedule.weekday;
    if (simpleSchedule.monthday) form.elements.schedule_monthday.value = simpleSchedule.monthday;
    if (simpleSchedule.interval) form.elements.schedule_interval.value = simpleSchedule.interval;
  }
  updateScheduleBuilder();
  data.sources.forEach((value) => appendRow("source", value));
  data.remotes.forEach((value) => appendRow("remote", value));
  updateArchiveHint();
  updateRetentionControls();
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

const simpleScheduleKinds = new Set(["daily", "weekly", "monthly", "every_hours", "every_minutes", "every_seconds"]);

function parseSimpleSchedule(schedule) {
  const fields = schedule.trim().split(/\s+/);
  const normalized = fields.length === 5 ? ["0", ...fields] : fields.length === 6 ? fields : null;
  if (!normalized) return null;
  const [second, minute, hour, monthday, month, weekday] = normalized;
  if (month !== "*") return null;
  if (/^(?:[0-9]|[1-5]\d)$/.test(minute) && /^(?:[0-9]|1\d|2[0-3])$/.test(hour)) {
    const time = `${hour.padStart(2, "0")}:${minute.padStart(2, "0")}`;
    if (second === "0" && monthday === "*" && weekday === "*") return { kind: "daily", time };
    if (second === "0" && monthday === "*" && /^(MON|TUE|WED|THU|FRI|SAT|SUN)$/.test(weekday)) return { kind: "weekly", time, weekday };
    if (second === "0" && /^([1-9]|[12]\d|3[01])$/.test(monthday) && weekday === "*") return { kind: "monthly", time, monthday };
  }
  let match = second.match(/^0\/([1-9]|[1-5]\d)$/);
  if (match && minute === "*" && hour === "*" && monthday === "*" && weekday === "*") return { kind: "every_seconds", interval: match[1] };
  match = minute.match(/^0\/([1-9]|[1-5]\d)$/);
  if (second === "0" && match && hour === "*" && monthday === "*" && weekday === "*") return { kind: "every_minutes", interval: match[1] };
  match = hour.match(/^0\/([1-9]|1\d|2[0-3])$/);
  if (second === "0" && minute === "0" && match && monthday === "*" && weekday === "*") return { kind: "every_hours", interval: match[1] };
  return null;
}

function buildSimpleSchedule(form) {
  const kind = form.elements.schedule_kind.value;
  const [hour, minute] = form.elements.schedule_time.value.split(":");
  const interval = Number(form.elements.schedule_interval.value);
  if (kind === "daily") return `0 ${Number(minute)} ${Number(hour)} * * *`;
  if (kind === "weekly") return `0 ${Number(minute)} ${Number(hour)} * * ${form.elements.schedule_weekday.value}`;
  if (kind === "monthly") return `0 ${Number(minute)} ${Number(hour)} ${Number(form.elements.schedule_monthday.value)} * *`;
  if (kind === "every_hours") return `0 0 0/${interval} * * *`;
  if (kind === "every_minutes") return `0 0/${interval} * * * *`;
  return `0/${interval} * * * * *`;
}

function simpleScheduleSummary(form) {
  const kind = form.elements.schedule_kind.value;
  const time = form.elements.schedule_time.value;
  const interval = form.elements.schedule_interval.value;
  const timezone = form.elements.timezone.value.trim() || "UTC";
  const weekdayKeys = { MON: "monday", TUE: "tuesday", WED: "wednesday", THU: "thursday", FRI: "friday", SAT: "saturday", SUN: "sunday" };
  const weekday = t(weekdayKeys[form.elements.schedule_weekday.value] || "monday");
  if (state.language === "zh") {
    if (kind === "daily") return `每天 ${time} · ${timezone}`;
    if (kind === "weekly") return `每周${weekday} ${time} · ${timezone}`;
    if (kind === "monthly") return `每月 ${form.elements.schedule_monthday.value} 日 ${time} · ${timezone}`;
    return `每 ${interval} ${t(kind === "every_hours" ? "hoursUnit" : kind === "every_minutes" ? "minutesUnit" : "secondsUnit")} · ${timezone}`;
  }
  if (kind === "daily") return `Daily at ${time} · ${timezone}`;
  if (kind === "weekly") return `Every ${weekday} at ${time} · ${timezone}`;
  if (kind === "monthly") return `Monthly on day ${form.elements.schedule_monthday.value} at ${time} · ${timezone}`;
  return `Every ${interval} ${t(kind === "every_hours" ? "hoursUnit" : kind === "every_minutes" ? "minutesUnit" : "secondsUnit")} · ${timezone}`;
}

function updateScheduleBuilder() {
  const form = $("#planForm");
  const simple = form.elements.schedule_mode.value === "simple";
  $("#simpleScheduleFields").hidden = !simple;
  $("#cronScheduleField").hidden = simple;
  form.elements.schedule.required = !simple;
  if (!simple) {
    form.elements.schedule_time.required = false;
    form.elements.schedule_weekday.required = false;
    form.elements.schedule_monthday.required = false;
    form.elements.schedule_interval.required = false;
    return;
  }
  const kind = form.elements.schedule_kind.value;
  if (!simpleScheduleKinds.has(kind)) form.elements.schedule_kind.value = "daily";
  const currentKind = form.elements.schedule_kind.value;
  const timed = ["daily", "weekly", "monthly"].includes(currentKind);
  $("[data-schedule-field=\"time\"]").hidden = !timed;
  $("[data-schedule-field=\"weekday\"]").hidden = currentKind !== "weekly";
  $("[data-schedule-field=\"monthday\"]").hidden = currentKind !== "monthly";
  $("[data-schedule-field=\"interval\"]").hidden = timed;
  form.elements.schedule_time.required = timed;
  form.elements.schedule_weekday.required = currentKind === "weekly";
  form.elements.schedule_monthday.required = currentKind === "monthly";
  form.elements.schedule_interval.required = !timed;
  const limits = currentKind === "every_hours" ? [23, "hoursUnit"] : currentKind === "every_minutes" ? [59, "minutesUnit"] : [59, "secondsUnit"];
  form.elements.schedule_interval.max = limits[0];
  if (Number(form.elements.schedule_interval.value) > limits[0]) form.elements.schedule_interval.value = limits[0];
  $("#scheduleIntervalUnit").textContent = t(limits[1]);
  const schedule = buildSimpleSchedule(form);
  form.elements.schedule.value = schedule;
  $("#generatedSchedule").textContent = schedule;
  $("#scheduleSummary").textContent = simpleScheduleSummary(form);
}

function collectPlan() {
  const form = $("#planForm");
  if (!form.reportValidity()) throw new Error(t("formInvalid"));
  const value = (name) => form.elements[name]?.value?.trim() || "";
  const number = (name) => Number(form.elements[name]?.value || 0);
  return {
    name: value("name"), enabled: form.elements.enabled.checked, schedule: form.elements.schedule_mode.value === "simple" ? buildSimpleSchedule(form) : form.elements.schedule.value, timezone: value("timezone"),
    sources: $$(".source-row").map((row) => ({ name: $('[data-field="name"]', row).value.trim(), path: $('[data-field="path"]', row).value.trim() })),
    archive: { kind: value("archive_kind"), password: value("archive_password") || (form.elements.archive_password.dataset.passwordSet === "true" ? "••••••••" : ""), password_hint: value("archive_password_hint"), suffix: value("archive_suffix") },
    remotes: $$(".remote-row").map((row) => ({ name: $('[data-field="name"]', row).value.trim(), directory: $('[data-field="directory"]', row).value.trim() })),
    retention: {
      keep_days: form.elements.keep_days.disabled ? 0 : number("keep_days"),
      keep_count: form.elements.keep_count.disabled ? 0 : number("keep_count"),
    },
    retry: { max_attempts: number("retry_count") + 1, initial_delay_seconds: number("initial_delay"), max_delay_seconds: number("max_delay"), backoff: value("backoff") },
    notifications: { ping: {}, mail: {}, serverchan: {} },
    rclone_flags: splitArgs(value("rclone_flags")),
    remote_check_concurrency: number("remote_check_concurrency"),
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
  $("#advancedProvider").hidden = true;
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
  $("#advancedProvider").hidden = true;
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
  const button = event.target.closest("[data-action], .nav-item, #newPlanButton, #configureRemoteButton, #addAccountButton, #themeButton, #languageButton, #menuButton, #refreshButton");
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
  if (button.id === "menuButton") setMenuOpen(!document.body.classList.contains("menu-open"));
  if (button.matches("[data-page-link]")) {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    const pointerTriggered = event.detail > 0 && navigationPointer?.link === button;
    navigationPointer = null;
    navigateToPage(button.dataset.pageLink, {
      push: true,
      animate: pointerTriggered,
      focusHeading: !pointerTriggered,
      resetScroll: true,
    });
  }
  if (button.id === "refreshButton") loadAll();
  if (["close", "close-log", "close-remote"].includes(action)) {
    button.closest("dialog").close();
    if (action === "close-log") state.openRunId = null;
  }
  if (action === "add-source") appendRow("source");
  if (action === "add-remote") appendRow("remote");
  if (action === "remove-row") button.closest(".repeat-row").remove();
  const card = button.closest(".plan-card");
  if (card && action === "edit") openPlan(state.plans.find((plan) => plan.id === card.dataset.id));
  if (card && action === "run") {
    button.disabled = true;
    try { const result = await api(`/api/plans/${card.dataset.id}/run`, { method: "POST" }); toast(t("runQueued")); await loadAll(); state.openRunId = result.run_id; $("#logDialog").showModal(); renderOpenRun(); }
    catch (error) { toast(error.message, true); } finally { button.disabled = false; }
  }
  if (card && action === "delete" && confirm(t("deleteConfirm"))) {
    try { await api(`/api/plans/${card.dataset.id}`, { method: "DELETE" }); toast(t("deleteSuccess")); await loadAll(); }
    catch (error) { toast(error.message, true); }
  }
  if (action === "log") {
    const run = state.runs.find((item) => item.id === button.closest(".history-row").dataset.run);
    state.openRunId = run?.id || null; $("#logDialog").showModal(); renderOpenRun();
  }
  if (action === "toggle-password") {
    const input = button.closest(".password-field").querySelector("input");
    if (!input.value && input.dataset.passwordSet === "true") {
      if (!state.status?.authentication_enabled) { toast(t("passwordRevealNeedsAuth"), true); return; }
      button.disabled = true;
      try {
        const result = await api(`/api/plans/${state.editingId}/archive-password`, { method: "POST" });
        if (typeof result?.password !== "string") throw new Error(t("loadError"));
        input.value = result.password;
      } catch (error) { toast(error.message, true); return; }
      finally { button.disabled = false; }
    }
    const visible = input.type === "password";
    input.type = visible ? "text" : "password";
    button.setAttribute("aria-pressed", String(visible));
    button.setAttribute("aria-label", t(visible ? "hidePassword" : "showPassword"));
    button.innerHTML = icon(visible ? "eye-off" : "eye");
    input.focus();
  }
  if (action === "add-notification") {
    if (state.notificationTargets.length >= 32) return;
    const selected = $("#notificationType").value;
    const id = crypto.randomUUID();
    const [type, channel] = selected.startsWith("serverchan-") ? ["serverchan", selected.split("-")[1]] : [selected, null];
    const defaults = type === "ping" ? { completion_url: "", start_url: "", success_url: "", failure_url: "" }
      : type === "email" ? { smtp_options: [], to: "" }
      : type === "serverchan" ? { channel, send_key: "" }
      : { server: "https://ntfy.sh", topic: "", token: "" };
    const baseName = type === "serverchan" ? t(channel === "app" ? "serverChanApp" : "serverChanWechat") : type === "email" ? "Email" : type === "ntfy" ? "ntfy" : "Ping";
    state.notificationTargets.push({ id, name: baseName, type, enabled: true, on_start: false, on_success: true, on_failure: true, config: defaults });
    state.expandedNotificationId = id;
    renderNotificationTargets();
    requestAnimationFrame(() => $(`[data-id="${CSS.escape(id)}"] [data-notification-name]`)?.focus());
  }
  const targetRow = button.closest(".notification-target");
  if (targetRow && action === "toggle-notification") {
    state.expandedNotificationId = state.expandedNotificationId === targetRow.dataset.id ? null : targetRow.dataset.id;
    renderNotificationTargets();
  }
  if (targetRow && action === "remove-notification") {
    const target = state.notificationTargets.find((item) => item.id === targetRow.dataset.id);
    if (target && confirm(`${t("removeNotification")}: ${target.name}?`)) {
      state.notificationTargets = state.notificationTargets.filter((item) => item.id !== target.id);
      state.expandedNotificationId = null;
      renderNotificationTargets();
    }
  }
  if (action === "test-notification") {
    const original = button.textContent;
    const status = targetRow?.querySelector(".target-test-status");
    button.disabled = true; button.textContent = t("testingNotification");
    try { await api("/api/notifications/test", { method: "POST", body: JSON.stringify({ target_id: button.dataset.id, config: collectNotifications() }) }); if (status) { status.textContent = t("notificationTestSuccess"); status.dataset.state = "success"; } }
    catch (error) { if (status) { status.textContent = `${t("testFailed")}: ${error.message}`; status.dataset.state = "error"; } } finally { button.disabled = false; button.textContent = original; }
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
  const securityHint = t(kind === "7z" ? "secureArchiveHint" : kind === "zip" ? "compatibleArchiveHint" : "nativeDirectoryHint");
  $("#archiveSecurityHint").textContent = `${form.elements.archive_password.dataset.passwordSet === "true" ? `${t("passwordAlreadySet")} ` : ""}${securityHint}`;
  form.elements.archive_password.disabled = kind === "none";
  updatePasswordToggle();
  updateRetentionControls();
}

function updatePasswordToggle() {
  const form = $("#planForm");
  const canRevealExisting = form.elements.archive_password.dataset.passwordSet === "true" && state.status?.authentication_enabled;
  $("[data-action=\"toggle-password\"]", form).disabled = form.elements.archive_password.disabled || (!form.elements.archive_password.value && !canRevealExisting);
}

function updateRetentionControls() {
  const form = $("#planForm");
  const archiveEnabled = form.elements.archive_kind.value !== "none";
  for (const name of ["keep_days", "keep_count"]) {
    const toggle = form.elements[`${name}_enabled`];
    const input = form.elements[name];
    toggle.disabled = !archiveEnabled;
    input.disabled = !archiveEnabled || !toggle.checked;
  }
}

document.addEventListener("click", (event) => {
  if (document.body.classList.contains("menu-open") && !event.target.closest(".sidebar, #menuButton")) setMenuOpen(false);
});
document.addEventListener("pointerdown", (event) => {
  const link = event.target.closest("[data-page-link]");
  navigationPointer = link ? { link, pointerType: event.pointerType } : null;
});
$("#notificationForm").addEventListener("input", (event) => {
  const row = event.target.closest(".notification-target");
  if (!row) return;
  const target = state.notificationTargets.find((item) => item.id === row.dataset.id);
  if (!target) return;
  if (event.target.matches("[data-notification-name]")) target.name = event.target.value;
  if (event.target.matches("[data-notification-enabled]")) target.enabled = event.target.checked;
  if (event.target.matches("[data-notification-event]")) target[`on_${event.target.dataset.notificationEvent}`] = event.target.checked;
  if (event.target.matches("[data-notification-field]")) {
    const field = event.target.dataset.notificationField;
    target.config[field] = field === "smtp_options" ? splitArgs(event.target.value) : event.target.value.trim();
  }
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && document.body.classList.contains("menu-open")) { setMenuOpen(false); $("#menuButton").focus(); }
});
window.addEventListener("popstate", () => {
  navigateToPage(pageFromPath(), { animate: false, focusHeading: false, resetScroll: false });
});
window.addEventListener("hashchange", () => {
  const page = legacyHashPage();
  if (!page) return;
  history.replaceState({ page }, "", `${pageRoutes[page].path}${location.search}`);
  navigateToPage(page, { animate: false, focusHeading: false, resetScroll: false });
});
$("#planForm").addEventListener("submit", savePlan);
$("#remoteForm").addEventListener("submit", saveRemote);
$("#notificationForm").addEventListener("submit", saveNotifications);
$("#providerSelect").addEventListener("change", selectProvider);
$("#planForm").elements.archive_kind.addEventListener("change", updateArchiveHint);
$("#planForm").elements.archive_password.addEventListener("input", updatePasswordToggle);
$("#planForm").addEventListener("change", (event) => {
  if (["keep_days_enabled", "keep_count_enabled"].includes(event.target.name)) updateRetentionControls();
});
$("#planForm").addEventListener("input", (event) => {
  if (["schedule_mode", "schedule_kind", "schedule_time", "schedule_weekday", "schedule_monthday", "schedule_interval", "timezone"].includes(event.target.name)) updateScheduleBuilder();
});
$("#planForm").addEventListener("change", (event) => {
  if (["schedule_mode", "schedule_kind", "schedule_weekday"].includes(event.target.name)) updateScheduleBuilder();
});
$$("dialog").forEach((dialog) => dialog.addEventListener("click", (event) => {
  const rect = dialog.getBoundingClientRect();
  if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) dialog.close();
}));

initializeNavigation();
applyPreferences();
loadAll();
setInterval(async () => {
  if (document.visibilityState === "visible") {
    try {
      state.status = await api("/api/status");
      state.runs = await api("/api/runs?limit=50");
      renderStatus(); renderMetrics(); renderRuns(); renderOpenRun();
    } catch {}
  }
}, 5000);
