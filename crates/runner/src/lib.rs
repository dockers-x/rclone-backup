use anyhow::{Context, anyhow, bail};
use chrono::{Local, Utc};
use chrono_tz::Tz;
use rclone_backup_core::*;
use rclone_backup_rclone::{
    RcloneRc, command_cancellation_unconfirmed, command_cancelled, command_timed_out,
};
use rclone_backup_store::Store;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::Command,
    sync::{Mutex, watch},
    task::JoinSet,
    time::{Duration, Instant, sleep_until},
};
use tracing::warn;
use uuid::Uuid;

const MAX_LOG_BYTES: usize = 512 * 1024;
const REMOTE_CHECK_TIMEOUT: Duration = Duration::from_secs(25);
const MANAGED_WORKSPACE_DIR: &str = ".runs";
const RUN_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
struct ActiveRun {
    run_id: String,
    cancel: watch::Sender<bool>,
}

#[derive(Clone)]
struct RunAttempt<'a> {
    id: &'a str,
    number: u32,
    control: RunControl,
}

struct BackupAttemptResult {
    result: anyhow::Result<()>,
    deferred_cleanup: Option<PathBuf>,
}

#[derive(Clone)]
struct RunControl {
    cancellation: watch::Receiver<bool>,
    deadline: Instant,
}

impl RunControl {
    fn new(cancellation: watch::Receiver<bool>) -> Self {
        Self::with_timeout(cancellation, RUN_TIMEOUT)
    }

    fn with_timeout(cancellation: watch::Receiver<bool>, duration: Duration) -> Self {
        Self {
            cancellation,
            deadline: Instant::now() + duration,
        }
    }

    fn ensure_running(&self) -> anyhow::Result<()> {
        if *self.cancellation.borrow() {
            return Err(RunCancelled.into());
        }
        if Instant::now() >= self.deadline {
            return Err(RunTimedOut.into());
        }
        Ok(())
    }

    fn remaining(&self, maximum: Duration) -> anyhow::Result<Duration> {
        self.ensure_running()?;
        Ok(maximum.min(self.deadline.saturating_duration_since(Instant::now())))
    }

    async fn wait(&mut self, duration: Duration) -> anyhow::Result<()> {
        self.ensure_running()?;
        let delay = sleep_until((Instant::now() + duration).min(self.deadline));
        tokio::pin!(delay);
        tokio::select! {
            () = &mut delay => {
                if Instant::now() >= self.deadline {
                    Err(RunTimedOut.into())
                } else {
                    Ok(())
                }
            }
            () = wait_for_run_cancellation(&mut self.cancellation) => Err(RunCancelled.into()),
        }
    }
}

#[derive(Debug)]
struct RunCancelled;

impl std::fmt::Display for RunCancelled {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("backup run was cancelled")
    }
}

impl std::error::Error for RunCancelled {}

#[derive(Debug)]
struct RunTimedOut;

impl std::fmt::Display for RunTimedOut {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("backup run exceeded its 24-hour timeout")
    }
}

impl std::error::Error for RunTimedOut {}

fn run_cancelled(error: &anyhow::Error) -> bool {
    error.downcast_ref::<RunCancelled>().is_some() || command_cancelled(error)
}

fn run_timed_out(error: &anyhow::Error) -> bool {
    error.downcast_ref::<RunTimedOut>().is_some() || command_timed_out(error)
}

#[derive(Debug)]
struct RcJobStateUncertain(String);

impl std::fmt::Display for RcJobStateUncertain {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "rclone job state is uncertain: {}", self.0)
    }
}

impl std::error::Error for RcJobStateUncertain {}

fn rc_job_state_uncertain(error: &anyhow::Error) -> bool {
    error.downcast_ref::<RcJobStateUncertain>().is_some()
}

#[derive(Clone)]
pub struct Runner {
    store: Store,
    work_dir: PathBuf,
    active: Arc<Mutex<HashMap<Uuid, ActiveRun>>>,
    rc: RcloneRc,
}

impl Runner {
    pub fn new(store: Store, work_dir: impl Into<PathBuf>, rc: RcloneRc) -> Self {
        Self {
            store,
            work_dir: work_dir.into(),
            active: Arc::new(Mutex::new(HashMap::new())),
            rc,
        }
    }

    pub async fn prepare_workspaces(work_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let work_dir = work_dir.as_ref();
        fs::create_dir_all(work_dir)
            .await
            .context("create workspace root")?;
        let managed = work_dir.join(MANAGED_WORKSPACE_DIR);
        if let Ok(metadata) = fs::symlink_metadata(&managed).await {
            if metadata.file_type().is_symlink() {
                bail!("managed workspace directory cannot be a symlink");
            }
            fs::remove_dir_all(&managed)
                .await
                .context("clear workspaces preserved by the previous daemon")?;
        }
        create_private_dir(&managed).await
    }

    pub async fn is_active(&self, id: Uuid) -> bool {
        self.active.lock().await.contains_key(&id)
    }

    pub async fn cancel_run(&self, run_id: &str) -> bool {
        let active = self.active.lock().await;
        request_cancellation(&active, run_id)
    }

    pub fn rclone_ready(&self) -> bool {
        !self.rclone_quarantined() && self.rc.is_ready()
    }

    pub async fn rc_refresh_ready(&self) -> bool {
        if self.rclone_quarantined() {
            false
        } else {
            self.rc.refresh_ready().await
        }
    }

    pub fn rclone_quarantined(&self) -> bool {
        self.rc.is_quarantined()
    }

    pub async fn rclone_stats(&self) -> serde_json::Value {
        self.rc
            .stats()
            .await
            .unwrap_or_else(|_| serde_json::json!({}))
    }

    pub fn rclone_version(&self) -> Option<&str> {
        self.rc.version()
    }

    pub async fn rclone_providers(&self) -> anyhow::Result<serde_json::Value> {
        self.rc.providers().await
    }

    pub async fn rclone_remotes(&self) -> anyhow::Result<serde_json::Value> {
        self.rc.remote_summaries().await
    }

    pub async fn create_rclone_remote(
        &self,
        name: &str,
        provider_type: &str,
        parameters: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.rc.create_remote(name, provider_type, parameters).await
    }

    pub async fn continue_rclone_remote(
        &self,
        name: &str,
        provider_type: &str,
        parameters: serde_json::Value,
        state: &str,
        result: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.rc
            .continue_remote(name, provider_type, parameters, state, result)
            .await
    }

    pub async fn update_rclone_remote(
        &self,
        name: &str,
        parameters: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.rc.update_remote(name, parameters).await
    }

    pub async fn delete_rclone_remote(&self, name: &str) -> anyhow::Result<()> {
        self.rc.delete_remote(name).await.map(|_| ())
    }

    pub async fn test_rclone_remote(&self, name: &str) -> anyhow::Result<()> {
        self.ensure_rclone_available()?;
        self.rc
            .run_command_with_timeout(
                "lsd",
                vec![format!("{name}:")],
                vec![
                    "--contimeout=8s".into(),
                    "--timeout=15s".into(),
                    "--retries=1".into(),
                    "--low-level-retries=1".into(),
                ],
                REMOTE_CHECK_TIMEOUT,
            )
            .await?;
        Ok(())
    }

    pub async fn test_notification(
        &self,
        config: &NotificationConfig,
        target_id: &str,
    ) -> anyhow::Result<()> {
        let existing = self
            .store
            .notification_settings()
            .await
            .map(|settings| settings.config)
            .unwrap_or_default();
        let mut config = config.clone();
        config.merge_redacted_from(&existing);
        config
            .validate_network_targets()
            .await
            .map_err(anyhow::Error::msg)?;
        if let Some(target) = config.targets.iter().find(|target| target.id == target_id) {
            let test = target_test_config(&config, target);
            test.validate().map_err(anyhow::Error::msg)?;
            let time = Local::now().format("%Y-%m-%d %H:%M:%S %:z").to_string();
            let report = rclone_backup_notifications::deliver_with_variables(
                "Rclone Backup Test",
                &test,
                "success",
                rclone_backup_notifications::NotificationVariables {
                    content_default: "Notification test from Rclone Backup",
                    content_en: "Notification test from Rclone Backup",
                    content_zh: "来自 Rclone Backup 的通知测试",
                    time: &time,
                    backup_size_bytes: Some(1_610_612_736),
                },
            )
            .await;
            if report.failed {
                bail!("notification delivery failed; check the server log and configuration");
            }
            return Ok(());
        }
        let mut test = NotificationConfig::default();
        let event = match target_id {
            "ping" => {
                test.ping = config.ping.clone();
                test.ping.enabled = true;
                if !test.ping.success_url.is_empty() || !test.ping.completion_url.is_empty() {
                    test.ping.on_success = true;
                    "success"
                } else if !test.ping.start_url.is_empty() {
                    test.ping.on_start = true;
                    "start"
                } else if !test.ping.failure_url.is_empty() {
                    test.ping.on_failure = true;
                    "failure"
                } else {
                    bail!("Ping URL is not configured");
                }
            }
            "mail" => {
                test.mail = config.mail.clone();
                test.mail.enabled = true;
                test.mail.on_success = true;
                "success"
            }
            "serverchan" => {
                test.serverchan = config.serverchan.clone();
                test.serverchan.enabled = true;
                test.serverchan.on_success = true;
                "success"
            }
            _ => bail!("notification target was not found"),
        };
        test.validate().map_err(anyhow::Error::msg)?;
        let mut log = LogBuffer::notification_test(&test);
        send_notification(
            "Rclone Backup Test",
            &test,
            None,
            NotificationEvent {
                name: event,
                content_en: "Notification test from Rclone Backup",
                content_zh: "来自 Rclone Backup 的通知测试",
                backup_size_bytes: None,
            },
            &mut log,
        )
        .await;
        if log.text.contains(" warning:") || log.text.contains("ServerChan warning:") {
            bail!("notification delivery failed; check the server log and configuration");
        }
        Ok(())
    }

    pub async fn start(self, plan: Plan, trigger: &str) -> anyhow::Result<String> {
        self.start_with_limit(plan, trigger, None)
            .await?
            .ok_or_else(|| anyhow!("unlimited backup start was skipped"))
    }

    pub async fn start_scheduled(
        self,
        plan: Plan,
        trigger: &str,
        max_active: usize,
    ) -> anyhow::Result<Option<String>> {
        self.start_with_limit(plan, trigger, Some(max_active)).await
    }

    async fn start_with_limit(
        self,
        plan: Plan,
        trigger: &str,
        max_active: Option<usize>,
    ) -> anyhow::Result<Option<String>> {
        self.ensure_rclone_available()?;
        if !self.rc.is_ready() {
            bail!("RCLONE_NOT_READY: configure at least one rclone remote first");
        }
        {
            let mut active = self.active.lock().await;
            if active.contains_key(&plan.id) {
                bail!("plan is already running");
            }
            if max_active.is_some_and(|limit| active.len() >= limit) {
                return Ok(None);
            }
            let (cancel, _) = watch::channel(false);
            active.insert(
                plan.id,
                ActiveRun {
                    run_id: String::new(),
                    cancel,
                },
            );
        }
        let run = match self.store.start_run(&plan, trigger).await {
            Ok(run) => run,
            Err(error) => {
                self.active.lock().await.remove(&plan.id);
                return Err(error);
            }
        };
        let cancellation = {
            let mut active = self.active.lock().await;
            let control = active
                .get_mut(&plan.id)
                .expect("active run exists after its history record is created");
            control.run_id.clone_from(&run.id);
            control.cancel.subscribe()
        };
        let run_id = run.id.clone();
        tokio::spawn(async move {
            if let Err(error) = self.execute(plan, run, cancellation).await {
                warn!(%error, "backup run failed");
            }
        });
        Ok(Some(run_id))
    }

    pub async fn execute_sync(&self, plan: Plan, trigger: &str) -> anyhow::Result<()> {
        self.ensure_rclone_available()?;
        if !self.rc.refresh_ready().await {
            bail!("RCLONE_NOT_READY: configure at least one rclone remote first");
        }
        {
            let mut active = self.active.lock().await;
            if active.contains_key(&plan.id) {
                bail!("plan is already running");
            }
            let (cancel, _) = watch::channel(false);
            active.insert(
                plan.id,
                ActiveRun {
                    run_id: String::new(),
                    cancel,
                },
            );
        }
        let run = match self.store.start_run(&plan, trigger).await {
            Ok(run) => run,
            Err(error) => {
                self.active.lock().await.remove(&plan.id);
                return Err(error);
            }
        };
        let cancellation = {
            let mut active = self.active.lock().await;
            let control = active
                .get_mut(&plan.id)
                .expect("active run exists after its history record is created");
            control.run_id.clone_from(&run.id);
            control.cancel.subscribe()
        };
        self.clone().execute(plan, run, cancellation).await
    }

    async fn execute(
        self,
        plan: Plan,
        run: RunRecord,
        cancellation: watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        let plan_id = plan.id;
        let result = self
            .execute_inner(&plan, &run, RunControl::new(cancellation))
            .await;
        self.active.lock().await.remove(&plan_id);
        result
    }

    fn ensure_rclone_available(&self) -> anyhow::Result<()> {
        if self.rclone_quarantined() {
            bail!(
                "RCLONE_STATE_UNCERTAIN: a previous rclone job may still be active; restart the service before starting another backup"
            );
        }
        Ok(())
    }

    async fn execute_inner(
        &self,
        plan: &Plan,
        run: &RunRecord,
        mut control: RunControl,
    ) -> anyhow::Result<()> {
        let notifications = match self.store.confirmed_notifications().await {
            Ok(config) => config.unwrap_or_default(),
            Err(error) => {
                warn!(%error, "cannot load global notifications; continuing without them");
                NotificationConfig::default()
            }
        };
        let mut log = LogBuffer::new(plan, &notifications);
        let mut result = Err(anyhow!("backup did not run"));
        let mut final_attempt = 1;
        let mut backup_size_bytes = None;
        let mut deferred_cleanup = None;
        for attempt in 1..=plan.retry.max_attempts {
            backup_size_bytes = None;
            if let Err(error) = control.ensure_running() {
                result = Err(error);
                break;
            }
            final_attempt = attempt;
            log.line(format!("Attempt {attempt}/{}", plan.retry.max_attempts));
            for remote in &plan.remotes {
                log.target(remote, "pending", "");
            }
            log.phase("checking_destinations");
            self.store
                .update_run(&run.id, "running", attempt, &log.text, false)
                .await?;
            let BackupAttemptResult {
                result: attempt_result,
                deferred_cleanup: cleanup,
            } = self
                .backup_once(
                    plan,
                    &notifications,
                    RunAttempt {
                        id: &run.id,
                        number: attempt,
                        control: control.clone(),
                    },
                    &mut backup_size_bytes,
                    &mut log,
                )
                .await;
            result = attempt_result;
            if cleanup.is_some() {
                deferred_cleanup = cleanup;
            }
            if result.is_ok() {
                break;
            }
            if result.as_ref().is_err_and(rc_job_state_uncertain) {
                self.rc.quarantine().await;
                log.line(
                    "Rclone job state could not be confirmed. The workspace was preserved and new backups are blocked until the service restarts.",
                );
                break;
            }
            if result.as_ref().is_err_and(run_cancelled)
                || result.as_ref().is_err_and(run_timed_out)
            {
                break;
            }
            if attempt < plan.retry.max_attempts {
                let delay = plan.retry.delay_for(attempt);
                log.line(format!(
                    "Attempt failed: {}. Retrying in {delay}s.",
                    result.as_ref().unwrap_err()
                ));
                self.store
                    .update_run(&run.id, "retrying", attempt, &log.text, false)
                    .await?;
                if let Err(error) = control.wait(Duration::from_secs(delay)).await {
                    result = Err(error);
                    break;
                }
            }
        }
        let cancelled = result.as_ref().is_err_and(run_cancelled);
        let timed_out = result.as_ref().is_err_and(run_timed_out);
        let status = if result.is_ok() {
            "success"
        } else if cancelled {
            "cancelled"
        } else if timed_out {
            "timed_out"
        } else {
            "failed"
        };
        if cancelled {
            log.phase("cancelled");
            log.line("Backup cancelled by user.");
        } else if let Err(error) = &result {
            log.phase(if timed_out { "timed_out" } else { "failed" });
            log.line(format!("Backup failed: {error:#}"));
            if timed_out {
                log.line("Failure notification skipped because the run deadline expired.");
            } else if let Err(notification_error) = notify_with_control(
                plan,
                &notifications,
                NotificationEvent {
                    name: "failure",
                    content_en: &format!("Backup failed. Reason: {error:#}"),
                    content_zh: &format!("备份失败。原因：{error:#}"),
                    backup_size_bytes,
                },
                &mut control,
                &mut log,
            )
            .await
            {
                log.line(format!(
                    "Failure notification stopped during finalization: {notification_error:#}"
                ));
            }
        } else {
            log.phase("completed");
            log.line(format!(
                "Backup completed successfully: {}/{} destinations.",
                plan.remotes.len(),
                plan.remotes.len()
            ));
        }
        self.store
            .update_run(&run.id, status, final_attempt, &log.text, true)
            .await?;
        if let Some(run_dir) = deferred_cleanup
            && let Err(error) = fs::remove_dir_all(&run_dir).await
        {
            warn!(%error, path = %run_dir.display(), "cleanup cancelled backup workspace");
        }
        result
    }

    async fn backup_once(
        &self,
        plan: &Plan,
        notifications: &NotificationConfig,
        run: RunAttempt<'_>,
        backup_size_bytes: &mut Option<u64>,
        log: &mut LogBuffer,
    ) -> BackupAttemptResult {
        let run_dir = self.work_dir.join(MANAGED_WORKSPACE_DIR).join(format!(
            "{}-{}",
            safe_name(&plan.name),
            Uuid::new_v4()
        ));
        if let Err(error) = create_private_dir(&run_dir)
            .await
            .context("create working directory")
        {
            return BackupAttemptResult {
                result: Err(error),
                deferred_cleanup: None,
            };
        }
        let result = self
            .backup_in_dir(plan, notifications, &run_dir, run, backup_size_bytes, log)
            .await;
        if result.as_ref().is_err_and(rc_job_state_uncertain) {
            log.line(format!(
                "Preserved workspace {} because rclone may still be reading it.",
                run_dir.display()
            ));
            return BackupAttemptResult {
                result,
                deferred_cleanup: None,
            };
        }
        if result.as_ref().is_err_and(run_cancelled) || result.as_ref().is_err_and(run_timed_out) {
            return BackupAttemptResult {
                result,
                deferred_cleanup: Some(run_dir),
            };
        }
        if let Err(error) = fs::remove_dir_all(&run_dir).await {
            warn!(%error, "cleanup working directory");
        }
        BackupAttemptResult {
            result,
            deferred_cleanup: None,
        }
    }

    async fn backup_in_dir(
        &self,
        plan: &Plan,
        notifications: &NotificationConfig,
        run_dir: &Path,
        mut run: RunAttempt<'_>,
        backup_size_bytes: &mut Option<u64>,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        run.control.ensure_running()?;
        notify_with_control(
            plan,
            notifications,
            NotificationEvent {
                name: "start",
                content_en: "Backup started.",
                content_zh: "备份已开始。",
                backup_size_bytes: None,
            },
            &mut run.control,
            log,
        )
        .await?;
        self.ensure_workspace_outside_sources(plan, run_dir)?;
        self.check_remotes(plan, run.id, run.number, run.control.clone(), log)
            .await?;
        run.control.ensure_running()?;
        let suffix = Utc::now()
            .with_timezone(&plan_timezone(plan))
            .format(&plan.archive.suffix)
            .to_string()
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let available_sources = available_sources(plan, log);
        if available_sources.is_empty() {
            bail!("none of the configured source directories exist");
        }
        let archive_prefix = archive_prefix(plan);
        let archive_base = format!("{archive_prefix}{suffix}");
        let staging = run_dir.join("contents");
        log.phase(if plan.archive.kind == "none" {
            "preparing_files"
        } else {
            "creating_archive"
        });
        self.checkpoint(run.id, run.number, log).await?;
        let upload = match plan.archive.kind.as_str() {
            "none" => {
                stage_sources(&available_sources, &staging, &mut run.control, log).await?;
                staging.clone()
            }
            kind => {
                let target = run_dir.join(format!("{archive_base}.{kind}"));
                let mut command = CommandSpec::new("7z");
                if kind == "zip" {
                    command = command.args(["a", "-tzip", "-mx=9"]);
                    if !plan.archive.password.is_empty() {
                        command = command
                            .arg("-mem=ZipCrypto")
                            .arg("-p")
                            .stdin_secret(&plan.archive.password);
                    }
                } else {
                    command = command.args(["a", "-t7z", "-m0=lzma2", "-mx=9"]);
                    if !plan.archive.password.is_empty() {
                        command = command
                            .arg("-mhe=on")
                            .arg("-p")
                            .stdin_secret(&plan.archive.password);
                    }
                }
                if available_sources.len() == 1 {
                    command = command
                        .arg(target.as_os_str())
                        .arg(".")
                        .current_dir(&available_sources[0].path);
                } else {
                    stage_sources(&available_sources, &staging, &mut run.control, log).await?;
                    command = command
                        .arg(target.as_os_str())
                        .arg(".")
                        .current_dir(&staging);
                }
                command = command.secret(&plan.archive.password);
                run_command(command, &mut run.control, log).await?;
                target
            }
        };
        match backup_size(&upload, &mut run.control).await {
            Ok(size) => *backup_size_bytes = Some(size),
            Err(error) if run_cancelled(&error) || run_timed_out(&error) => return Err(error),
            Err(error) => log.line(format!("Backup size is unavailable: {error:#}")),
        }
        run.control.ensure_running()?;
        if plan.archive.kind != "none"
            && fs::try_exists(&staging).await.unwrap_or(false)
            && let Err(error) = fs::remove_dir_all(&staging).await
        {
            warn!(%error, "cleanup plaintext archive staging directory");
        }
        log.phase("uploading");
        for remote in &plan.remotes {
            log.target(remote, "pending", "");
        }
        self.checkpoint(run.id, run.number, log).await?;
        let concurrency = plan.upload_concurrency.clamp(1, MAX_UPLOAD_CONCURRENCY);
        let upload_context = UploadContext {
            rc: self.rc.clone(),
            source: upload.to_string_lossy().into_owned(),
            archive_kind: plan.archive.kind.clone(),
            archive_base: archive_base.clone(),
            flags: plan.rclone_flags.clone(),
            control: run.control.clone(),
        };
        let mut pending = plan.remotes.iter().cloned();
        let mut uploads = JoinSet::new();
        let mut upload_failures = Vec::new();
        let mut checkpoint_error = None;
        let mut uncertain_job = None;
        let mut cancellation_error = None;
        let mut timeout_error = None;

        while uploads.len() < concurrency && !self.rclone_quarantined() {
            let Some(remote) = pending.next() else { break };
            start_remote_upload(&mut uploads, remote, upload_context.clone(), log);
        }
        remember_first_error(
            &mut checkpoint_error,
            self.checkpoint(run.id, run.number, log).await,
        );

        while let Some(result) = uploads.join_next().await {
            match result {
                Ok((remote, destination, Ok(()))) => {
                    log.target(&remote, "success", "");
                    log.line(format!("Upload to {destination} succeeded."));
                }
                Ok((remote, destination, Err(error))) => {
                    let detail = format!("{error:#}");
                    let cancelled = run_cancelled(&error);
                    let timed_out = run_timed_out(&error);
                    let cancellation_unconfirmed = command_cancellation_unconfirmed(&error);
                    if cancelled {
                        log.target(&remote, "cancelled", &detail);
                        log.line(format!("Upload to {destination} cancelled."));
                        if cancellation_error.is_none() {
                            cancellation_error = Some(error);
                        }
                    } else if timed_out {
                        log.target(&remote, "timed_out", &detail);
                        log.line(format!("Upload to {destination} timed out."));
                        if timeout_error.is_none() {
                            timeout_error = Some(error);
                        }
                    } else {
                        log.target(&remote, "failed", &detail);
                        log.line(format!("Upload to {destination} failed: {detail}"));
                        upload_failures.push(remote.name);
                    }
                    if cancellation_unconfirmed && uncertain_job.is_none() {
                        self.rc.quarantine().await;
                        uncertain_job = Some(detail);
                    }
                }
                Err(error) => {
                    log.line(format!("An upload task stopped unexpectedly: {error}"));
                    upload_failures.push("unexpected task failure".into());
                }
            }
            remember_first_error(
                &mut checkpoint_error,
                self.checkpoint(run.id, run.number, log).await,
            );

            if checkpoint_error.is_none()
                && uncertain_job.is_none()
                && cancellation_error.is_none()
                && timeout_error.is_none()
                && !self.rclone_quarantined()
                && let Some(next) = pending.next()
            {
                start_remote_upload(&mut uploads, next, upload_context.clone(), log);
                remember_first_error(
                    &mut checkpoint_error,
                    self.checkpoint(run.id, run.number, log).await,
                );
            }
        }
        if uncertain_job.is_none() && self.rclone_quarantined() {
            uncertain_job = Some(
                "another rclone job entered quarantine while this upload queue was active".into(),
            );
        }
        if let Some(detail) = uncertain_job {
            let detail = if let Some(error) = checkpoint_error {
                format!("{detail}; progress persistence also failed: {error:#}")
            } else {
                detail
            };
            return Err(RcJobStateUncertain(detail).into());
        }
        if let Some(error) = checkpoint_error {
            return Err(error.context("persist upload progress after draining active jobs"));
        }
        if let Some(error) = cancellation_error {
            return Err(error);
        }
        if let Some(error) = timeout_error {
            return Err(error);
        }
        if !upload_failures.is_empty() {
            bail!(
                "upload failed for {}/{} destinations: {}",
                upload_failures.len(),
                plan.remotes.len(),
                upload_failures.join(", ")
            );
        }
        log.phase("retention");
        self.checkpoint(run.id, run.number, log).await?;
        run.control.ensure_running()?;
        self.apply_retention(plan, &mut run.control, log).await?;
        run.control.ensure_running()?;
        notify_with_control(
            plan,
            notifications,
            NotificationEvent {
                name: "success",
                content_en: "Backup completed successfully.",
                content_zh: "备份已成功完成。",
                backup_size_bytes: *backup_size_bytes,
            },
            &mut run.control,
            log,
        )
        .await?;
        Ok(())
    }

    async fn check_remotes(
        &self,
        plan: &Plan,
        run_id: &str,
        attempt: u32,
        control: RunControl,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        let concurrency = plan
            .remote_check_concurrency
            .clamp(1, MAX_REMOTE_CHECK_CONCURRENCY);
        let mut pending = plan.remotes.iter().cloned();
        let mut checks = JoinSet::new();
        let mut available = 0;
        let mut checkpoint_error = None;
        let mut uncertain_job = None;

        while checks.len() < concurrency && !self.rclone_quarantined() {
            let Some(remote) = pending.next() else { break };
            start_remote_check(
                &mut checks,
                remote,
                self.rc.clone(),
                plan.rclone_flags.clone(),
                control.clone(),
                log,
            );
        }
        remember_first_error(
            &mut checkpoint_error,
            self.checkpoint(run_id, attempt, log).await,
        );

        while let Some(result) = checks.join_next().await {
            match result {
                Ok((remote, mkdir_attempted, ready, detail, cancellation_uncertain)) => {
                    if mkdir_attempted {
                        log.line(format!("rclone mkdir {}", remote_path(&remote)));
                    }
                    if ready {
                        available += 1;
                        log.target(&remote, "ready", &detail);
                    } else {
                        log.target(&remote, "unavailable", &detail);
                        log.line(format!("Destination {} unavailable: {detail}", remote.name));
                    }
                    if cancellation_uncertain && uncertain_job.is_none() {
                        self.rc.quarantine().await;
                        uncertain_job = Some(detail);
                    }
                }
                Err(error) => log.line(format!(
                    "A destination check task stopped unexpectedly: {error}"
                )),
            }
            remember_first_error(
                &mut checkpoint_error,
                self.checkpoint(run_id, attempt, log).await,
            );
            if checkpoint_error.is_none()
                && let Err(error) = control.ensure_running()
            {
                checkpoint_error = Some(error);
            }

            if checkpoint_error.is_none()
                && uncertain_job.is_none()
                && !self.rclone_quarantined()
                && let Some(next) = pending.next()
            {
                start_remote_check(
                    &mut checks,
                    next,
                    self.rc.clone(),
                    plan.rclone_flags.clone(),
                    control.clone(),
                    log,
                );
                remember_first_error(
                    &mut checkpoint_error,
                    self.checkpoint(run_id, attempt, log).await,
                );
            }
        }
        if uncertain_job.is_none() && self.rclone_quarantined() {
            uncertain_job = Some(
                "another rclone job entered quarantine while destination checks were active".into(),
            );
        }
        if let Some(detail) = uncertain_job {
            let detail = if let Some(error) = checkpoint_error {
                format!("{detail}; progress persistence also failed: {error:#}")
            } else {
                detail
            };
            return Err(RcJobStateUncertain(detail).into());
        }
        if let Some(error) = checkpoint_error {
            return Err(error.context("persist destination checks after draining active jobs"));
        }
        control.ensure_running()?;
        if available == 0 {
            bail!("all rclone destinations are unavailable");
        }
        Ok(())
    }

    async fn checkpoint(&self, run_id: &str, attempt: u32, log: &LogBuffer) -> anyhow::Result<()> {
        self.store
            .update_run(run_id, "running", attempt, &log.text, false)
            .await
    }

    async fn apply_retention(
        &self,
        plan: &Plan,
        control: &mut RunControl,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        let archive_prefix = archive_prefix(plan);
        let include = format!("/{archive_prefix}*.{}", plan.archive.kind);
        for remote in &plan.remotes {
            control.ensure_running()?;
            if self.rclone_quarantined() {
                return Err(RcJobStateUncertain(
                    "rclone entered quarantine before retention completed".into(),
                )
                .into());
            }
            let destination = remote_path(remote);
            if plan.retention.keep_days > 0 {
                log.line(format!(
                    "rclone delete {destination} --min-age {}d",
                    plan.retention.keep_days
                ));
                if let Err(error) = self
                    .rc
                    .run_command_with_control(
                        "delete",
                        vec![
                            destination.clone(),
                            "--min-age".into(),
                            format!("{}d", plan.retention.keep_days),
                            "--include".into(),
                            include.clone(),
                        ],
                        plan.rclone_flags.clone(),
                        control.remaining(RUN_TIMEOUT)?,
                        &mut control.cancellation,
                    )
                    .await
                {
                    self.handle_retention_error("Age retention", error, log)
                        .await?;
                }
            }
            if plan.retention.keep_count > 0 {
                log.line(format!("rclone lsjson {destination} --files-only"));
                match self
                    .rc
                    .run_command_output_with_control(
                        "lsjson",
                        vec![destination.clone(), "--files-only".into()],
                        plan.rclone_flags.clone(),
                        control.remaining(RUN_TIMEOUT)?,
                        &mut control.cancellation,
                    )
                    .await
                    .and_then(|out| {
                        serde_json::from_str::<Vec<RcloneItem>>(&out).map_err(Into::into)
                    }) {
                    Ok(mut items) => {
                        items.retain(|item| {
                            item.path.starts_with(&archive_prefix)
                                && item.path.ends_with(&format!(".{}", plan.archive.kind))
                        });
                        items.sort_by(|a, b| b.mod_time.cmp(&a.mod_time));
                        for item in items.into_iter().skip(plan.retention.keep_count as usize) {
                            control.ensure_running()?;
                            let path = format!(
                                "{}/{}",
                                destination.trim_end_matches('/'),
                                item.path.trim_start_matches('/')
                            );
                            log.line(format!("rclone deletefile {path}"));
                            if let Err(error) = self
                                .rc
                                .run_command_with_control(
                                    "deletefile",
                                    vec![path],
                                    plan.rclone_flags.clone(),
                                    control.remaining(RUN_TIMEOUT)?,
                                    &mut control.cancellation,
                                )
                                .await
                            {
                                self.handle_retention_error("Count retention", error, log)
                                    .await?;
                            }
                        }
                    }
                    Err(error) => {
                        self.handle_retention_error("Count retention", error, log)
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_retention_error(
        &self,
        operation: &str,
        error: anyhow::Error,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        if command_cancellation_unconfirmed(&error) {
            self.rc.quarantine().await;
            return Err(RcJobStateUncertain(format!(
                "{operation} left an rclone job in an uncertain state: {error:#}"
            ))
            .into());
        }
        log.line(format!("{operation} warning: {error}"));
        Ok(())
    }

    fn ensure_workspace_outside_sources(&self, plan: &Plan, run_dir: &Path) -> anyhow::Result<()> {
        let run_dir = std::fs::canonicalize(run_dir).context("resolve working directory")?;
        for source in &plan.sources {
            let source_path = std::fs::canonicalize(&source.path)
                .with_context(|| format!("resolve source {}", source.path))?;
            if run_dir.starts_with(source_path) {
                bail!(
                    "working directory {} must be outside source {}",
                    self.work_dir.display(),
                    source.path
                );
            }
        }
        Ok(())
    }
}

fn request_cancellation(active: &HashMap<Uuid, ActiveRun>, run_id: &str) -> bool {
    active
        .values()
        .find(|run| run.run_id == run_id)
        .is_some_and(|run| run.cancel.send(true).is_ok())
}

fn target_test_config(
    config: &NotificationConfig,
    target: &rclone_backup_core::NotificationTarget,
) -> NotificationConfig {
    let mut target = target.clone();
    target.enabled = true;
    target.on_success = true;
    NotificationConfig {
        targets: vec![target],
        templates: config.templates.clone(),
        ..Default::default()
    }
}

async fn create_private_dir(path: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(path).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).await?;
    }
    Ok(())
}

fn archive_prefix(plan: &Plan) -> String {
    let id = plan.id.simple().to_string();
    format!("{}-{}-", safe_name(&plan.name), &id[..8])
}

fn available_sources<'a>(plan: &'a Plan, log: &mut LogBuffer) -> Vec<&'a FolderSource> {
    plan.sources
        .iter()
        .filter(|source| {
            if !Path::new(&source.path).is_dir() {
                log.line(format!("Source {} does not exist; skipped.", source.path));
                false
            } else {
                true
            }
        })
        .collect()
}

async fn stage_sources(
    sources: &[&FolderSource],
    staging: &Path,
    control: &mut RunControl,
    log: &mut LogBuffer,
) -> anyhow::Result<()> {
    control.ensure_running()?;
    fs::create_dir_all(staging).await?;
    if sources.len() == 1 {
        run_command(
            CommandSpec::new("cp")
                .args(["-a", "--"])
                .arg(format!("{}/.", sources[0].path.trim_end_matches('/')))
                .arg(staging.as_os_str()),
            control,
            log,
        )
        .await?;
    } else {
        for source in sources {
            control.ensure_running()?;
            let destination = staging.join(safe_name(&source.name));
            fs::create_dir_all(&destination).await?;
            run_command(
                CommandSpec::new("cp")
                    .args(["-a", "--"])
                    .arg(format!("{}/.", source.path.trim_end_matches('/')))
                    .arg(destination.as_os_str()),
                control,
                log,
            )
            .await?;
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct RcloneItem {
    #[serde(rename = "Path")]
    path: String,
    #[serde(rename = "ModTime")]
    mod_time: String,
}

fn remote_path(remote: &RemoteConfig) -> String {
    format!("{}:{}", remote.name, remote.directory.trim_end_matches('/'))
}

fn start_remote_check(
    checks: &mut JoinSet<(RemoteConfig, bool, bool, String, bool)>,
    remote: RemoteConfig,
    rc: RcloneRc,
    mut flags: Vec<String>,
    control: RunControl,
    log: &mut LogBuffer,
) {
    let destination = remote_path(&remote);
    log.target(&remote, "checking", "");
    log.line(format!("rclone lsd {destination}"));
    flags.extend([
        "--contimeout=8s".into(),
        "--timeout=15s".into(),
        "--retries=1".into(),
        "--low-level-retries=1".into(),
    ]);
    checks.spawn(async move {
        let mut control = control;
        let check = rc
            .run_command_with_control(
                "lsd",
                vec![destination.clone()],
                flags.clone(),
                match control.remaining(REMOTE_CHECK_TIMEOUT) {
                    Ok(duration) => duration,
                    Err(error) => {
                        return (remote, false, false, format!("{error:#}"), false);
                    }
                },
                &mut control.cancellation,
            )
            .await;
        match check {
            Ok(_) => (remote, false, true, String::new(), false),
            Err(error) if command_timed_out(&error) => {
                let cancellation_uncertain = command_cancellation_unconfirmed(&error);
                (remote, false, false, error.to_string(), cancellation_uncertain)
            }
            Err(error) if command_cancellation_unconfirmed(&error) => {
                (remote, false, false, format!("{error:#}"), true)
            }
            Err(check_error) => {
                let create = rc
                    .run_command_with_control(
                        "mkdir",
                        vec![destination],
                        flags,
                        match control.remaining(REMOTE_CHECK_TIMEOUT) {
                            Ok(duration) => duration,
                            Err(error) => {
                                return (remote, false, false, format!("{error:#}"), false);
                            }
                        },
                        &mut control.cancellation,
                    )
                    .await;
                match create {
                    Ok(_) => (remote, true, true, "directory created".into(), false),
                    Err(create_error) => (
                        remote,
                        true,
                        false,
                        format!(
                            "connection check failed: {check_error:#}; directory creation failed: {create_error:#}"
                        ),
                        command_cancellation_unconfirmed(&create_error),
                    ),
                }
            }
        }
    });
}

fn remember_first_error(slot: &mut Option<anyhow::Error>, result: anyhow::Result<()>) {
    if slot.is_none()
        && let Err(error) = result
    {
        *slot = Some(error);
    }
}

#[derive(Clone)]
struct UploadContext {
    rc: RcloneRc,
    source: String,
    archive_kind: String,
    archive_base: String,
    flags: Vec<String>,
    control: RunControl,
}

fn start_remote_upload(
    uploads: &mut JoinSet<(RemoteConfig, String, anyhow::Result<()>)>,
    remote: RemoteConfig,
    context: UploadContext,
    log: &mut LogBuffer,
) {
    let destination = if context.archive_kind == "none" {
        format!(
            "{}/{}",
            remote_path(&remote).trim_end_matches('/'),
            context.archive_base,
        )
    } else {
        remote_path(&remote)
    };
    log.target(&remote, "uploading", "");
    log.line(format!("rclone copy {} {destination}", context.source));
    uploads.spawn(async move {
        let mut control = context.control;
        let duration = match control.remaining(RUN_TIMEOUT) {
            Ok(duration) => duration,
            Err(error) => return (remote, destination, Err(error)),
        };
        let result = context
            .rc
            .run_command_with_control(
                "copy",
                vec![context.source, destination.clone()],
                context.flags,
                duration,
                &mut control.cancellation,
            )
            .await
            .map(|_| ())
            .with_context(|| format!("upload to {destination}"));
        (remote, destination, result)
    });
}

#[derive(Clone)]
struct CommandSpec {
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    current_dir: Option<PathBuf>,
    stdin: Option<String>,
    secrets: Vec<String>,
}

impl CommandSpec {
    fn new(program: &str) -> Self {
        Self {
            program: program.into(),
            args: vec![],
            env: HashMap::new(),
            current_dir: None,
            stdin: None,
            secrets: vec![],
        }
    }
    fn arg(mut self, value: impl AsRef<std::ffi::OsStr>) -> Self {
        self.args
            .push(value.as_ref().to_string_lossy().into_owned());
        self
    }
    fn args<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.args.extend(
            values
                .into_iter()
                .map(|v| v.as_ref().to_string_lossy().into_owned()),
        );
        self
    }
    fn current_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(value.into());
        self
    }
    fn stdin_secret(mut self, value: &str) -> Self {
        self.stdin = Some(format!("{value}\n{value}\n"));
        self.secret(value)
    }
    fn secret(mut self, value: &str) -> Self {
        if !value.is_empty() {
            self.secrets.push(value.into());
        }
        self
    }
}

async fn run_command(
    spec: CommandSpec,
    control: &mut RunControl,
    log: &mut LogBuffer,
) -> anyhow::Result<()> {
    capture_command(spec, control, log).await.map(|_| ())
}

async fn capture_command(
    spec: CommandSpec,
    control: &mut RunControl,
    log: &mut LogBuffer,
) -> anyhow::Result<String> {
    control.ensure_running()?;
    let rendered = redact(
        &format!("$ {} {}", spec.program, spec.args.join(" ")),
        &spec.secrets,
    );
    log.line(rendered);
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .envs(&spec.env)
        .stderr(Stdio::piped());
    command.stdin(if spec.stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    command.stdout(Stdio::piped());
    command.kill_on_drop(true);
    if let Some(directory) = &spec.current_dir {
        command.current_dir(directory);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("start {}", spec.program))?;
    if let Some(input) = &spec.stdin
        && let Some(stdin) = &mut child.stdin
    {
        stdin.write_all(input.as_bytes()).await?;
    }
    drop(child.stdin.take());
    let deadline = control.deadline;
    let output = tokio::select! {
        output = child.wait_with_output() => output?,
        () = wait_for_run_cancellation(&mut control.cancellation) => {
            return Err(RunCancelled.into());
        }
        () = sleep_until(deadline) => {
            return Err(RunTimedOut.into());
        }
    };
    let stdout = redact(&String::from_utf8_lossy(&output.stdout), &spec.secrets);
    let stderr = redact(&String::from_utf8_lossy(&output.stderr), &spec.secrets);
    if !stdout.trim().is_empty() {
        log.line(stdout.trim());
    }
    if !stderr.trim().is_empty() {
        log.line(stderr.trim());
    }
    if !output.status.success() {
        bail!("{} exited with {}", spec.program, output.status);
    }
    Ok(stdout)
}

#[derive(Clone, Copy)]
struct NotificationEvent<'a> {
    name: &'a str,
    content_en: &'a str,
    content_zh: &'a str,
    backup_size_bytes: Option<u64>,
}

async fn send_notification(
    plan_name: &str,
    notifications: &NotificationConfig,
    timezone: Option<&str>,
    event: NotificationEvent<'_>,
    log: &mut LogBuffer,
) {
    let time = if let Some(timezone) = timezone.and_then(|value| value.parse::<Tz>().ok()) {
        Utc::now()
            .with_timezone(&timezone)
            .format("%Y-%m-%d %H:%M:%S %:z")
            .to_string()
    } else {
        Local::now().format("%Y-%m-%d %H:%M:%S %:z").to_string()
    };
    let legacy_time = Local::now();
    let content_default = match event.name {
        "start" => format!("Start backup at {legacy_time}"),
        "success" => format!("Backup completed at {legacy_time}"),
        "failure" => format!(
            "Backup failed at {legacy_time}. Reason: {}",
            event
                .content_en
                .strip_prefix("Backup failed. Reason: ")
                .unwrap_or(event.content_en)
        ),
        _ => event.content_en.to_owned(),
    };
    let report = rclone_backup_notifications::deliver_with_variables(
        plan_name,
        notifications,
        event.name,
        rclone_backup_notifications::NotificationVariables {
            content_default: &content_default,
            content_en: event.content_en,
            content_zh: event.content_zh,
            time: &time,
            backup_size_bytes: event.backup_size_bytes,
        },
    )
    .await;
    for message in report.messages {
        log.line(message);
    }
}

async fn notify(
    plan: &Plan,
    notifications: &NotificationConfig,
    event: NotificationEvent<'_>,
    log: &mut LogBuffer,
) {
    send_notification(&plan.name, notifications, Some(&plan.timezone), event, log).await;
}

async fn notify_with_control(
    plan: &Plan,
    notifications: &NotificationConfig,
    event: NotificationEvent<'_>,
    control: &mut RunControl,
    log: &mut LogBuffer,
) -> anyhow::Result<()> {
    control.ensure_running()?;
    tokio::select! {
        () = notify(
            plan,
            notifications,
            event,
            log,
        ) => Ok(()),
        () = wait_for_run_cancellation(&mut control.cancellation) => Err(RunCancelled.into()),
        () = sleep_until(control.deadline) => Err(RunTimedOut.into()),
    }
}

fn plan_timezone(plan: &Plan) -> Tz {
    plan.timezone.parse().unwrap_or(chrono_tz::UTC)
}

async fn backup_size(path: &Path, control: &mut RunControl) -> anyhow::Result<u64> {
    let mut pending = vec![path.to_owned()];
    let mut total = 0_u64;
    while let Some(path) = pending.pop() {
        let metadata = controlled_fs(control, fs::symlink_metadata(&path)).await?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
            continue;
        }
        let mut entries = controlled_fs(control, fs::read_dir(&path)).await?;
        while let Some(entry) = controlled_fs(control, entries.next_entry()).await? {
            pending.push(entry.path());
        }
    }
    Ok(total)
}

async fn controlled_fs<T>(
    control: &mut RunControl,
    operation: impl std::future::Future<Output = std::io::Result<T>>,
) -> anyhow::Result<T> {
    control.ensure_running()?;
    tokio::select! {
        result = operation => Ok(result?),
        () = wait_for_run_cancellation(&mut control.cancellation) => Err(RunCancelled.into()),
        () = sleep_until(control.deadline) => Err(RunTimedOut.into()),
    }
}

async fn wait_for_run_cancellation(cancellation: &mut watch::Receiver<bool>) {
    loop {
        if *cancellation.borrow() {
            return;
        }
        if cancellation.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

fn safe_name(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    value
        .trim_matches('-')
        .chars()
        .take(80)
        .collect::<String>()
        .to_ascii_lowercase()
}
fn redact(value: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|s| !s.is_empty())
        .fold(value.to_owned(), |out, secret| {
            out.replace(secret, REDACTED)
        })
}

struct LogBuffer {
    text: String,
    secrets: Vec<String>,
    targets: std::collections::BTreeMap<String, serde_json::Value>,
}
impl LogBuffer {
    fn new(plan: &Plan, notifications: &NotificationConfig) -> Self {
        let mut secrets = vec![
            plan.archive.password.clone(),
            notifications.serverchan.send_key.clone(),
        ];
        secrets.extend([
            notifications.ping.completion_url.clone(),
            notifications.ping.start_url.clone(),
            notifications.ping.success_url.clone(),
            notifications.ping.failure_url.clone(),
        ]);
        secrets.extend(notifications.mail.smtp_options.iter().filter_map(|value| {
            let lowercase = value.to_ascii_lowercase();
            ["password", "pass=", "token", "secret"]
                .iter()
                .any(|marker| lowercase.contains(marker))
                .then(|| value.clone())
        }));
        secrets.extend(
            notifications
                .ping
                .completion_options
                .iter()
                .chain(&notifications.ping.start_options)
                .chain(&notifications.ping.success_options)
                .chain(&notifications.ping.failure_options)
                .cloned(),
        );
        secrets.extend(notifications.mail.smtp_options.iter().filter_map(|value| {
            value
                .split_once('=')
                .map(|(_, secret)| secret.to_owned())
                .filter(|secret| !secret.is_empty())
        }));
        secrets.push(notifications.mail.to.clone());
        for target in &notifications.targets {
            match &target.kind {
                NotificationTargetKind::Ping { config } => {
                    secrets.extend([
                        config.completion_url.clone(),
                        config.start_url.clone(),
                        config.success_url.clone(),
                        config.failure_url.clone(),
                    ]);
                    secrets.extend(
                        config
                            .completion_options
                            .iter()
                            .chain(&config.start_options)
                            .chain(&config.success_options)
                            .chain(&config.failure_options)
                            .cloned(),
                    );
                }
                NotificationTargetKind::Email { config } => {
                    secrets.extend([
                        config.password.clone(),
                        config.username.clone(),
                        config.from.clone(),
                    ]);
                    secrets.push(config.to.clone());
                }
                NotificationTargetKind::ServerChan { config } => {
                    secrets.push(config.send_key.clone())
                }
                NotificationTargetKind::Ntfy { config } => secrets.push(config.token.clone()),
            }
        }
        Self {
            text: String::new(),
            secrets,
            targets: std::collections::BTreeMap::new(),
        }
    }
    fn notification_test(notifications: &NotificationConfig) -> Self {
        let empty_plan = Plan {
            id: Uuid::nil(),
            name: String::new(),
            enabled: false,
            schedule: String::new(),
            timezone: String::new(),
            sources: Vec::new(),
            archive: ArchiveConfig::default(),
            remotes: Vec::new(),
            retention: RetentionPolicy::default(),
            retry: RetryPolicy::default(),
            notifications: NotificationConfig::default(),
            rclone_flags: Vec::new(),
            remote_check_concurrency: DEFAULT_REMOTE_CHECK_CONCURRENCY,
            upload_concurrency: DEFAULT_UPLOAD_CONCURRENCY,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        Self::new(&empty_plan, notifications)
    }
    fn line(&mut self, value: impl AsRef<str>) {
        let line = redact(value.as_ref(), &self.secrets);
        let entry = format!("[{}] {}\n", Local::now().format("%Y-%m-%d %H:%M:%S"), line);
        let required = self
            .text
            .len()
            .saturating_add(entry.len())
            .saturating_sub(MAX_LOG_BYTES);
        if required > 0 {
            let boundary = self.text.ceil_char_boundary(required.min(self.text.len()));
            self.text.drain(..boundary);
        }
        self.text.push_str(&entry);
        if self.text.len() > MAX_LOG_BYTES {
            let boundary = self
                .text
                .ceil_char_boundary(self.text.len() - MAX_LOG_BYTES);
            self.text.drain(..boundary);
        }
    }

    fn phase(&mut self, phase: &str) {
        self.event(serde_json::json!({
            "kind": "phase",
            "phase": phase,
            "at": chrono::Utc::now().to_rfc3339(),
        }));
        for target in self.targets.values().cloned().collect::<Vec<_>>() {
            self.event(target);
        }
    }

    fn target(&mut self, remote: &RemoteConfig, status: &str, detail: &str) {
        let event = serde_json::json!({
            "kind": "target",
            "name": remote.name,
            "directory": remote.directory,
            "status": status,
            "detail": detail,
            "at": chrono::Utc::now().to_rfc3339(),
        });
        self.targets.insert(
            format!("{}\0{}", remote.name, remote.directory),
            event.clone(),
        );
        self.event(event);
    }

    fn event(&mut self, value: serde_json::Value) {
        self.line(format!("@event {value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_run_can_be_cancelled_by_persistent_run_id() {
        let plan_id = Uuid::new_v4();
        let (cancel, mut cancellation) = watch::channel(false);
        let active = HashMap::from([(
            plan_id,
            ActiveRun {
                run_id: "run-123".into(),
                cancel,
            },
        )]);

        assert!(request_cancellation(&active, "run-123"));
        assert!(*cancellation.borrow_and_update());
        assert!(!request_cancellation(&active, "missing"));
    }

    #[tokio::test]
    async fn run_control_interrupts_retry_waits_and_enforces_deadline() {
        let (cancel, cancellation) = watch::channel(false);
        let mut control = RunControl::with_timeout(cancellation, Duration::from_secs(60));
        cancel.send(true).unwrap();
        let error = control.wait(Duration::from_secs(60)).await.unwrap_err();
        assert!(run_cancelled(&error));

        let (_cancel, cancellation) = watch::channel(false);
        let mut control = RunControl::with_timeout(cancellation, Duration::from_millis(1));
        let error = control.wait(Duration::from_secs(60)).await.unwrap_err();
        assert!(run_timed_out(&error));
    }

    #[tokio::test]
    async fn backup_size_counts_nested_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("one"), [0_u8; 10]).unwrap();
        std::fs::create_dir(directory.path().join("nested")).unwrap();
        std::fs::write(directory.path().join("nested/two"), [0_u8; 7]).unwrap();

        let (_cancel, cancellation) = watch::channel(false);
        let mut control = RunControl::new(cancellation);
        assert_eq!(
            backup_size(directory.path(), &mut control).await.unwrap(),
            17
        );
    }

    #[test]
    fn names_and_secrets_are_safe() {
        assert_eq!(safe_name("My Files / 1"), "my-files---1");
        assert_eq!(redact("token=secret", &["secret".into()]), "token=••••••••");
    }

    #[test]
    fn target_notification_test_keeps_selected_template() {
        let event = rclone_backup_core::NotificationEventTemplate {
            title: "{{plan_name}} custom".into(),
            body: "{{content}}".into(),
        };
        let target = rclone_backup_core::NotificationTarget {
            id: "ntfy-test".into(),
            name: "ntfy".into(),
            template_id: "custom".into(),
            enabled: true,
            on_start: false,
            on_success: true,
            on_failure: true,
            kind: rclone_backup_core::NotificationTargetKind::Ntfy {
                config: rclone_backup_core::NtfyTargetConfig {
                    server: "https://ntfy.sh".into(),
                    topic: "backup".into(),
                    token: String::new(),
                },
            },
        };
        let config = NotificationConfig {
            targets: vec![target.clone()],
            templates: vec![rclone_backup_core::NotificationTemplate {
                id: "custom".into(),
                name: "Custom".into(),
                language: "en".into(),
                start: event.clone(),
                success: event.clone(),
                failure: event,
            }],
            ..Default::default()
        };

        let test = target_test_config(&config, &target);

        assert_eq!(test.templates, config.templates);
        assert!(test.validate().is_ok());
    }

    #[tokio::test]
    async fn managed_workspaces_are_private_and_stale_runs_are_removed() {
        let directory = tempfile::tempdir().unwrap();
        let work_dir = directory.path().join("work");
        let stale = work_dir.join(MANAGED_WORKSPACE_DIR).join("stale");
        fs::create_dir_all(&stale).await.unwrap();
        fs::write(stale.join("plaintext"), b"secret").await.unwrap();

        Runner::prepare_workspaces(&work_dir).await.unwrap();

        assert!(!stale.exists());
        let managed = work_dir.join(MANAGED_WORKSPACE_DIR);
        assert!(managed.is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&managed).await.unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn managed_workspace_cleanup_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let work_dir = directory.path().join("work");
        let outside = directory.path().join("outside");
        fs::create_dir_all(&work_dir).await.unwrap();
        fs::create_dir_all(&outside).await.unwrap();
        symlink(&outside, work_dir.join(MANAGED_WORKSPACE_DIR)).unwrap();

        assert!(
            Runner::prepare_workspaces(&work_dir)
                .await
                .unwrap_err()
                .to_string()
                .contains("cannot be a symlink")
        );
        assert!(outside.is_dir());
    }

    #[test]
    fn progress_events_are_structured_and_redacted() {
        let remote = RemoteConfig {
            name: "cloud".into(),
            directory: "/backup".into(),
        };
        let mut log = LogBuffer {
            text: String::new(),
            secrets: vec!["secret".into()],
            targets: Default::default(),
        };

        log.target(&remote, "failed", "token=secret");

        let event = log
            .text
            .lines()
            .next()
            .unwrap()
            .split_once("@event ")
            .unwrap()
            .1;
        let event: serde_json::Value = serde_json::from_str(event).unwrap();
        assert_eq!(event["kind"], "target");
        assert_eq!(event["name"], remote.name);
        assert!(
            chrono::DateTime::parse_from_rfc3339(event["at"].as_str().unwrap()).is_ok(),
            "progress events need a machine-readable timestamp for truthful elapsed time"
        );
        assert!(!log.text.contains("secret"));
        assert!(log.text.contains(REDACTED));
    }

    #[test]
    fn progress_events_survive_a_full_log_buffer() {
        let remote = RemoteConfig {
            name: "cloud".into(),
            directory: "/backup".into(),
        };
        let mut log = LogBuffer {
            text: "x".repeat(MAX_LOG_BYTES),
            secrets: vec![],
            targets: Default::default(),
        };
        log.target(&remote, "success", "");
        log.phase("completed");
        assert!(log.text.len() <= MAX_LOG_BYTES);
        assert!(log.text.contains(r#""status":"success""#));
        assert!(log.text.contains(r#""phase":"completed""#));
    }

    #[test]
    fn notification_options_are_log_secrets() {
        let mut config = NotificationConfig::default();
        config.ping.success_options = vec![
            "--header".into(),
            "Authorization: Bearer private-token".into(),
        ];
        config.mail.smtp_options =
            vec!["-S".into(), "mta=smtps://alice:hunter2@smtp.example".into()];
        let mut log = LogBuffer::notification_test(&config);
        log.line("Authorization: Bearer private-token");
        log.line("mta=smtps://alice:hunter2@smtp.example");
        assert!(!log.text.contains("private-token"));
        assert!(!log.text.contains("hunter2"));
    }

    #[tokio::test]
    async fn notification_curl_protocol_arguments_are_accepted() {
        let command = CommandSpec::new("curl").args([
            "--proto",
            "=https",
            "--noproxy",
            "*",
            "--connect-timeout",
            "1",
            "--max-time",
            "1",
            "https://127.0.0.1:9/",
        ]);
        let output = Command::new(&command.program)
            .args(&command.args)
            .output()
            .await
            .expect("curl must be installed for notification delivery");
        assert_ne!(output.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("badly used"));
    }

    #[test]
    fn smtp_server_detection_supports_split_options() {
        assert_eq!(smtp_server_from_options(&[]), None);
        assert_eq!(
            smtp_server_from_options(&["-S".into(), "mta=smtps://smtp.example".into()]),
            Some("smtps://smtp.example")
        );
    }

    #[tokio::test]
    async fn single_source_staging_extracts_to_contents_directly() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let staging = directory.path().join("staging");
        fs::create_dir_all(source.join("nested")).await.unwrap();
        fs::write(source.join("root.txt"), "root").await.unwrap();
        fs::write(source.join("nested/file.txt"), "nested")
            .await
            .unwrap();
        let sources = [FolderSource {
            name: "data".into(),
            path: source.to_string_lossy().into_owned(),
        }];
        let refs = [&sources[0]];
        let mut log = LogBuffer {
            text: String::new(),
            secrets: vec![],
            targets: Default::default(),
        };
        let (_cancel, cancellation) = watch::channel(false);
        let mut control = RunControl::new(cancellation);
        stage_sources(&refs, &staging, &mut control, &mut log)
            .await
            .unwrap();
        assert_eq!(
            fs::read_to_string(staging.join("root.txt")).await.unwrap(),
            "root"
        );
        assert_eq!(
            fs::read_to_string(staging.join("nested/file.txt"))
                .await
                .unwrap(),
            "nested"
        );
        assert!(!staging.join("data").exists());
    }

    #[tokio::test]
    async fn multiple_source_staging_keeps_named_top_level_directories() {
        let directory = tempfile::tempdir().unwrap();
        let one = directory.path().join("one");
        let two = directory.path().join("two");
        let staging = directory.path().join("staging");
        fs::create_dir_all(&one).await.unwrap();
        fs::create_dir_all(&two).await.unwrap();
        fs::write(one.join("a.txt"), "a").await.unwrap();
        fs::write(two.join("b.txt"), "b").await.unwrap();
        let sources = [
            FolderSource {
                name: "Primary Data".into(),
                path: one.to_string_lossy().into_owned(),
            },
            FolderSource {
                name: "Photos".into(),
                path: two.to_string_lossy().into_owned(),
            },
        ];
        let refs = [&sources[0], &sources[1]];
        let mut log = LogBuffer {
            text: String::new(),
            secrets: vec![],
            targets: Default::default(),
        };
        let (_cancel, cancellation) = watch::channel(false);
        let mut control = RunControl::new(cancellation);
        stage_sources(&refs, &staging, &mut control, &mut log)
            .await
            .unwrap();
        assert_eq!(
            fs::read_to_string(staging.join("primary-data/a.txt"))
                .await
                .unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(staging.join("photos/b.txt"))
                .await
                .unwrap(),
            "b"
        );
    }
}
