import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  Archive,
  HardDrive,
  Globe2,
  Bell,
  FileText,
  History,
  Sun,
  Moon,
  Monitor,
  Plus,
  Menu,
  X,
  ArrowUpRight,
  RefreshCw,
} from "lucide-react";
import { MotionConfig } from "motion/react";
import { OvioProvider } from "./components/shared/world-provider";
import { OvioToaster, ovioToast } from "./components/ovio/toast/toast";
import {
  Button,
  IconButton,
  ErrorNotice,
  Loading,
  Confirm,
} from "./components/ui";
import { translations } from "./lib/i18n";
import { api } from "./lib/api";
import { PlansPage } from "./pages/Plans";
import { AccountsPage } from "./pages/Accounts";
import { NotificationsPage, TemplatesPage } from "./pages/Notifications";
import { HistoryPage, RunModal } from "./pages/History";
import { PlanModal } from "./pages/PlanModal";
import { RemoteModal } from "./pages/RemoteModal";

const Context = createContext(null);
export const useApp = () => useContext(Context);
const routes = [
  ["plans", "backupPlans", HardDrive],
  ["accounts", "storageAccounts", Globe2],
  ["notifications", "notifications", Bell],
  ["templates", "notificationTemplates", FileText],
  ["history", "history", History],
];
function currentPage() {
  const key =
    location.hash.slice(1) || location.pathname.replace(/^\/+|\/+$/g, "");
  return routes.some(([page]) => page === key) ? key : "plans";
}
export default function App() {
  const [language, setLanguage] = useState(() =>
    ["zh", "en"].includes(localStorage.getItem("language"))
      ? localStorage.getItem("language")
      : navigator.language.startsWith("zh")
        ? "zh"
        : "en",
  );
  const [theme, setTheme] = useState(() =>
    ["system", "light", "dark"].includes(localStorage.getItem("theme"))
      ? localStorage.getItem("theme")
      : "system",
  );
  const [world, setWorld] = useState(() =>
    ["minimal", "craft", "retro", "toy"].includes(localStorage.getItem("world"))
      ? localStorage.getItem("world")
      : "toy",
  );
  const [page, setPage] = useState(currentPage);
  const [menu, setMenu] = useState(false);
  const [data, setData] = useState(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [modal, setModal] = useState(null);
  const [confirmation, setConfirmation] = useState(null);
  const heading = useRef(null);
  const sequence = useRef(0);
  const inFlight = useRef(0);
  const pendingFocus = useRef(false);
  const t = useCallback(
    (key) => translations[language][key] || key,
    [language],
  );
  const say = (zh, en) => (language === "zh" ? zh : en);
  const siteName =
    document
      .querySelector('meta[name="site-name"]')
      ?.content.replace("{{SITE_NAME}}", "Rclone Backup") || "Rclone Backup";
  const refresh = useCallback(async (silent = false) => {
    const request = ++sequence.current;
    inFlight.current += 1;
    if (!silent) setLoading(true);
    try {
      const [
        status,
        plans,
        runs,
        remoteResponse,
        notifications,
        metrics,
        health,
      ] = await Promise.all(
        [
          "/api/status",
          "/api/plans",
          "/api/runs?include_logs=false",
          "/api/rclone/remotes",
          "/api/notifications",
          "/api/metrics",
          "/api/health",
        ].map((path) => api(path)),
      );
      if (request !== sequence.current) return;
      setData({
        status: { ...status, version: health.version },
        plans,
        runs,
        remotes: remoteResponse.remotes || [],
        notifications,
        metrics,
      });
      setError("");
    } catch (e) {
      if (request === sequence.current) setError(e.message);
    } finally {
      inFlight.current -= 1;
      if (request === sequence.current) setLoading(false);
    }
  }, []);
  useEffect(() => {
    refresh();
    const id = setInterval(() => {
      if (!document.hidden && !inFlight.current) refresh(true);
    }, 5000);
    return () => clearInterval(id);
  }, [refresh]);
  useEffect(() => {
    document.documentElement.dataset.world = world;
    localStorage.setItem("world", world);
  }, [world]);
  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
    localStorage.setItem("language", language);
  }, [language]);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("theme", theme);
  }, [theme]);
  useEffect(() => {
    const onPop = () => {
      setPage(currentPage());
      setMenu(false);
    };
    window.addEventListener("popstate", onPop);
    window.addEventListener("hashchange", onPop);
    if (location.hash)
      history.replaceState(null, "", `/${currentPage()}${location.search}`);
    return () => {
      window.removeEventListener("popstate", onPop);
      window.removeEventListener("hashchange", onPop);
    };
  }, []);
  useEffect(() => {
    if (pendingFocus.current && !menu) {
      heading.current?.focus();
      pendingFocus.current = false;
    }
  }, [page, menu]);
  const navigate = (next) => {
    if (next !== page) {
      history.pushState(null, "", `/${next}${location.search}`);
      pendingFocus.current = true;
      setPage(next);
      window.scrollTo({ top: 0, behavior: "instant" });
    }
    setMenu(false);
  };
  const notify = (message, isError = false) =>
    ovioToast[isError ? "error" : "success"](message);
  const action = async (task, message) => {
    try {
      const result = await task();
      if (message) notify(message);
      await refresh(true);
      return result;
    } catch (e) {
      notify(e.message, true);
      throw e;
    }
  };
  const context = {
    ...data,
    data,
    t,
    say,
    language,
    notify,
    refresh,
    action,
    openPlan: (plan) => setModal({ type: "plan", plan }),
    openRemote: (remote) => setModal({ type: "remote", remote }),
    openRun: (id) => setModal({ type: "run", id }),
    confirm: setConfirmation,
    navigate,
  };
  const ready = data?.status.rclone_ready && !data?.status.rclone_quarantined;
  const nav = (
    <>
      <div className="brand">
        <span className="brand-mark">
          <Archive size={23} />
        </span>
        <div>
          <strong title={siteName}>{siteName}</strong>
          <small>{say("备份工作空间", "Backup workspace")}</small>
        </div>
      </div>
      <div className="nav-label">{say("工作空间", "WORKSPACE")}</div>
      <nav aria-label={t("primaryNavigation")}>
        {routes.map(([key, label, Icon]) => (
          <a
            key={key}
            href={`/${key}`}
            aria-current={page === key ? "page" : undefined}
            className={`nav-item ${page === key ? "selected" : ""}`}
            onClick={(event) => {
              if (
                event.ctrlKey ||
                event.metaKey ||
                event.shiftKey ||
                event.altKey ||
                event.button !== 0
              )
                return;
              event.preventDefault();
              navigate(key);
            }}
          >
            <Icon size={18} />
            <span>{t(label)}</span>
            {key === "plans" && data && <small>{data.plans.length}</small>}
          </a>
        ))}
      </nav>
      <div className="sidebar-bottom">
        <div className="service-status">
          <BadgeStatus ready={ready} />
          <span>
            {data
              ? t(ready ? "serviceOnline" : "schedulerWaiting")
              : say("正在连接", "Connecting")}
          </span>
        </div>
        <div className="version-line">
          <span>rclone</span>
          <span>v{data?.status.version || "2.3.0"}</span>
        </div>
        <a
          className="docs-link"
          href="/api/docs"
          target="_blank"
          rel="noreferrer"
        >
          {say("API 文档", "API documentation")}
          <ArrowUpRight size={14} />
        </a>
      </div>
    </>
  );
  return (
    <Context.Provider value={context}>
      <MotionConfig reducedMotion="user">
        <OvioProvider world={world}>
          <a className="skip-link" href="#main">
            {t("skip")}
          </a>
          <div className="app-shell" data-ovio-world={world}>
            <aside className="sidebar">{nav}</aside>
            <div className="workspace">
              <header className="topbar">
                <div className="breadcrumbs">
                  <IconButton
                    icon={Menu}
                    label={t("menuOpen")}
                    className="icon-button mobile-menu"
                    onClick={() => setMenu(true)}
                  />
                  <span className="breadcrumb-root">
                    {say("工作空间", "Workspace")}
                  </span>
                  <span className="breadcrumb-divider">/</span>
                  <span>{t(routes.find(([key]) => key === page)?.[1])}</span>
                </div>
                <div className="top-actions">
                  <label className="world-picker">
                    <span className="sr-only">
                      {say("界面风格", "Interface style")}
                    </span>
                    <select
                      value={world}
                      onChange={(e) => setWorld(e.target.value)}
                      aria-label={say("界面风格", "Interface style")}
                    >
                      {[
                        ["minimal", say("极简", "Minimal")],
                        ["craft", say("手作", "Craft")],
                        ["retro", say("复古", "Retro")],
                        ["toy", say("玩具", "Toy")],
                      ].map(([value, label]) => (
                        <option key={value} value={value}>
                          {label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <IconButton
                    icon={{ system: Monitor, light: Sun, dark: Moon }[theme]}
                    label={`${t("themeToggle")} · ${say({ system: "跟随系统", light: "浅色", dark: "深色" }[theme], theme)}`}
                    onClick={() =>
                      setTheme(
                        { system: "light", light: "dark", dark: "system" }[
                          theme
                        ],
                      )
                    }
                  />
                  <Button
                    className="language-button"
                    onClick={() => setLanguage(language === "zh" ? "en" : "zh")}
                    aria-label={t("switchLanguage")}
                  >
                    <Globe2 size={16} />
                    {language === "zh" ? "中文" : "EN"}
                  </Button>
                </div>
              </header>
              <main id="main" tabIndex={-1}>
                <div className="page-heading">
                  <div>
                    <h1 ref={heading} tabIndex={-1}>
                      {t(routes.find(([key]) => key === page)?.[1])}
                    </h1>
                    <p>
                      {
                        {
                          plans: say(
                            "管理数据源、备份目标和自动运行计划。",
                            "Your sources, destinations, and automated schedules.",
                          ),
                          accounts: t("accountBoundary"),
                          notifications: t("globalNotificationHint"),
                          templates: t("templateLibraryHint"),
                          history: say(
                            "查看最近 50 次运行的状态、进度和完整日志。",
                            "Status, progress, and full logs for the 50 most recent runs.",
                          ),
                        }[page]
                      }
                    </p>
                  </div>
                  <div className="page-actions">
                    {page === "plans" && (
                      <Button
                        variant="primary"
                        onClick={() => setModal({ type: "plan" })}
                      >
                        <Plus size={16} />
                        {t("newPlan")}
                      </Button>
                    )}
                    {page === "accounts" && (
                      <Button
                        variant="primary"
                        onClick={() => setModal({ type: "remote" })}
                      >
                        <Plus size={16} />
                        {t("addAccount")}
                      </Button>
                    )}
                    {page === "history" && (
                      <Button busy={loading} onClick={() => refresh()}>
                        <RefreshCw size={16} />
                        {t("refresh")}
                      </Button>
                    )}
                  </div>
                </div>
                <ErrorNotice>
                  {error && (
                    <>
                      {t("loadError")}: {error}{" "}
                      <Button onClick={() => refresh()}>{t("refresh")}</Button>
                    </>
                  )}
                </ErrorNotice>
                {!data ? (
                  <Loading />
                ) : (
                  <>
                    <div hidden={page !== "plans"}>
                      <PlansPage />
                    </div>
                    <div hidden={page !== "accounts"}>
                      <AccountsPage />
                    </div>
                    <div hidden={page !== "notifications"}>
                      <NotificationsPage />
                    </div>
                    <div hidden={page !== "templates"}>
                      <TemplatesPage />
                    </div>
                    <div hidden={page !== "history"}>
                      <HistoryPage />
                    </div>
                  </>
                )}
              </main>
              <footer className="workspace-footer">
                <span>Rclone Backup</span>
                <span>
                  {say("数据在你手中。", "Your data, in your hands.")}
                </span>
              </footer>
            </div>
          </div>
          {menu && (
            <ModalNav
              onClose={() => setMenu(false)}
              title={t("primaryNavigation")}
              closeLabel={t("close")}
            >
              {nav}
            </ModalNav>
          )}
          {data && modal?.type === "plan" && (
            <PlanModal plan={modal.plan} onClose={() => setModal(null)} />
          )}
          {data && modal?.type === "remote" && (
            <RemoteModal remote={modal.remote} onClose={() => setModal(null)} />
          )}
          {data && modal?.type === "run" && (
            <RunModal id={modal.id} onClose={() => setModal(null)} />
          )}
          {confirmation && (
            <Confirm
              {...confirmation}
              t={t}
              onClose={() => setConfirmation(null)}
            />
          )}
          <OvioToaster
            position="bottom-right"
            containerAriaLabel={say("操作通知", "Notifications")}
          />
        </OvioProvider>
      </MotionConfig>
    </Context.Provider>
  );
}
function BadgeStatus({ ready }) {
  return (
    <span
      className={`status-dot ${ready ? "online" : ""}`}
      aria-hidden="true"
    />
  );
}
import * as Dialog from "@radix-ui/react-dialog";
function ModalNav({ children, onClose, title, closeLabel }) {
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="modal-overlay" />
        <Dialog.Content className="mobile-sidebar">
          <Dialog.Title className="sr-only">{title}</Dialog.Title>
          <Dialog.Description className="sr-only">{title}</Dialog.Description>
          <Dialog.Close asChild>
            <IconButton
              icon={X}
              label={closeLabel}
              className="icon-button nav-close"
            />
          </Dialog.Close>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
