use anyhow::{Context, anyhow, bail};
use chrono::Local;
use rclone_backup_core::*;
use rclone_backup_rclone::RcloneRc;
use rclone_backup_store::Store;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::Command,
    sync::{Mutex, Semaphore},
    time::{Duration, sleep},
};
use tracing::warn;
use uuid::Uuid;

const MAX_LOG_BYTES: usize = 512 * 1024;

#[derive(Clone)]
pub struct Runner {
    store: Store,
    work_dir: PathBuf,
    active: Arc<Mutex<HashSet<Uuid>>>,
    rc: RcloneRc,
}

impl Runner {
    pub fn new(store: Store, work_dir: impl Into<PathBuf>, rc: RcloneRc) -> Self {
        Self {
            store,
            work_dir: work_dir.into(),
            active: Arc::new(Mutex::new(HashSet::new())),
            rc,
        }
    }

    pub async fn is_active(&self, id: Uuid) -> bool {
        self.active.lock().await.contains(&id)
    }

    pub fn rclone_ready(&self) -> bool {
        self.rc.is_ready()
    }

    pub async fn rc_refresh_ready(&self) -> bool {
        self.rc.refresh_ready().await
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
        self.rc
            .run_command("lsd", vec![format!("{name}:")], vec![])
            .await
            .map(|_| ())
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
            let mut target = target.clone();
            target.enabled = true;
            target.on_success = true;
            let mut test = NotificationConfig::default();
            test.targets.push(target);
            test.validate().map_err(anyhow::Error::msg)?;
            let report = rclone_backup_notifications::deliver(
                "Rclone Backup Test",
                &test,
                "success",
                "Notification test from Rclone Backup",
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
            event,
            "Notification test from Rclone Backup",
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
        if !self.rc.is_ready() {
            bail!("RCLONE_NOT_READY: configure at least one rclone remote first");
        }
        {
            let mut active = self.active.lock().await;
            if active.contains(&plan.id) {
                bail!("plan is already running");
            }
            if max_active.is_some_and(|limit| active.len() >= limit) {
                return Ok(None);
            }
            active.insert(plan.id);
        }
        let run = match self.store.start_run(&plan, trigger).await {
            Ok(run) => run,
            Err(error) => {
                self.active.lock().await.remove(&plan.id);
                return Err(error);
            }
        };
        let run_id = run.id.clone();
        tokio::spawn(async move {
            if let Err(error) = self.execute(plan, run).await {
                warn!(%error, "backup run failed");
            }
        });
        Ok(Some(run_id))
    }

    pub async fn execute_sync(&self, plan: Plan, trigger: &str) -> anyhow::Result<()> {
        if !self.rc.refresh_ready().await {
            bail!("RCLONE_NOT_READY: configure at least one rclone remote first");
        }
        {
            let mut active = self.active.lock().await;
            if !active.insert(plan.id) {
                bail!("plan is already running");
            }
        }
        let run = match self.store.start_run(&plan, trigger).await {
            Ok(run) => run,
            Err(error) => {
                self.active.lock().await.remove(&plan.id);
                return Err(error);
            }
        };
        self.clone().execute(plan, run).await
    }

    async fn execute(self, plan: Plan, run: RunRecord) -> anyhow::Result<()> {
        let plan_id = plan.id;
        let result = self.execute_inner(&plan, &run).await;
        self.active.lock().await.remove(&plan_id);
        result
    }

    async fn execute_inner(&self, plan: &Plan, run: &RunRecord) -> anyhow::Result<()> {
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
        for attempt in 1..=plan.retry.max_attempts {
            final_attempt = attempt;
            log.line(format!("Attempt {attempt}/{}", plan.retry.max_attempts));
            for remote in &plan.remotes {
                log.target(remote, "pending", "");
            }
            log.phase("checking_destinations");
            self.store
                .update_run(&run.id, "running", attempt, &log.text, false)
                .await?;
            result = self
                .backup_once(plan, &notifications, &run.id, attempt, &mut log)
                .await;
            if result.is_ok() {
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
                sleep(Duration::from_secs(delay)).await;
            }
        }
        let status = if result.is_ok() { "success" } else { "failed" };
        if let Err(error) = &result {
            log.phase("failed");
            log.line(format!("Backup failed: {error:#}"));
            notify(
                plan,
                &notifications,
                "failure",
                &format!("Backup failed at {}. Reason: {error:#}", Local::now()),
                &mut log,
            )
            .await;
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
        result
    }

    async fn backup_once(
        &self,
        plan: &Plan,
        notifications: &NotificationConfig,
        run_id: &str,
        attempt: u32,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        let run_dir = self
            .work_dir
            .join(format!("{}-{}", safe_name(&plan.name), Uuid::new_v4()));
        fs::create_dir_all(&run_dir)
            .await
            .context("create working directory")?;
        let result = self
            .backup_in_dir(plan, notifications, &run_dir, run_id, attempt, log)
            .await;
        if let Err(error) = fs::remove_dir_all(&run_dir).await {
            warn!(%error, "cleanup working directory");
        }
        result
    }

    async fn backup_in_dir(
        &self,
        plan: &Plan,
        notifications: &NotificationConfig,
        run_dir: &Path,
        run_id: &str,
        attempt: u32,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        notify(
            plan,
            notifications,
            "start",
            &format!("Start backup at {}", Local::now()),
            log,
        )
        .await;
        self.ensure_workspace_outside_sources(plan, run_dir)?;
        self.check_remotes(plan, run_id, attempt, log).await?;
        let suffix = Local::now()
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
        self.checkpoint(run_id, attempt, log).await?;
        let upload = match plan.archive.kind.as_str() {
            "none" => {
                stage_sources(&available_sources, &staging, log).await?;
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
                    stage_sources(&available_sources, &staging, log).await?;
                    command = command
                        .arg(target.as_os_str())
                        .arg(".")
                        .current_dir(&staging);
                }
                command = command.secret(&plan.archive.password);
                run_command(command, log).await?;
                target
            }
        };
        log.phase("uploading");
        self.checkpoint(run_id, attempt, log).await?;
        let mut upload_failures = Vec::new();
        for remote in &plan.remotes {
            let destination = if plan.archive.kind == "none" {
                format!(
                    "{}/{archive_base}",
                    remote_path(remote).trim_end_matches('/')
                )
            } else {
                remote_path(remote)
            };
            log.target(remote, "uploading", "");
            log.line(format!("rclone copy {} {destination}", upload.display()));
            self.checkpoint(run_id, attempt, log).await?;
            match self
                .rc
                .run_command(
                    "copy",
                    vec![upload.to_string_lossy().into_owned(), destination.clone()],
                    plan.rclone_flags.clone(),
                )
                .await
                .with_context(|| format!("upload to {destination}"))
            {
                Ok(_) => {
                    log.target(remote, "success", "");
                    log.line(format!("Upload to {destination} succeeded."));
                }
                Err(error) => {
                    let detail = format!("{error:#}");
                    log.target(remote, "failed", &detail);
                    log.line(format!("Upload to {destination} failed: {detail}"));
                    upload_failures.push(remote.name.clone());
                }
            }
            self.checkpoint(run_id, attempt, log).await?;
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
        self.checkpoint(run_id, attempt, log).await?;
        self.apply_retention(plan, log).await;
        notify(
            plan,
            notifications,
            "success",
            &format!("Backup completed at {}", Local::now()),
            log,
        )
        .await;
        Ok(())
    }

    async fn check_remotes(
        &self,
        plan: &Plan,
        run_id: &str,
        attempt: u32,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        let concurrency = plan
            .remote_check_concurrency
            .clamp(1, MAX_REMOTE_CHECK_CONCURRENCY);
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut checks = Vec::with_capacity(plan.remotes.len());

        for remote in &plan.remotes {
            let destination = remote_path(remote);
            log.target(remote, "checking", "");
            log.line(format!("rclone lsd {destination}"));
            let rc = self.rc.clone();
            let flags = plan.rclone_flags.clone();
            let permit = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = permit.acquire_owned().await.ok();
                if rc
                    .run_command("lsd", vec![destination.clone()], flags.clone())
                    .await
                    .is_ok()
                {
                    return (false, true);
                }
                let created = rc
                    .run_command("mkdir", vec![destination], flags)
                    .await
                    .is_ok();
                (true, created)
            });
            checks.push((remote, handle));
        }
        self.checkpoint(run_id, attempt, log).await?;

        let mut available = 0;
        for (remote, check) in checks {
            let (mkdir_attempted, ready) = check.await.unwrap_or((false, false));
            if mkdir_attempted {
                log.line(format!("rclone mkdir {}", remote_path(remote)));
            }
            if ready {
                available += 1;
                log.target(remote, "ready", "");
            } else {
                log.target(
                    remote,
                    "unavailable",
                    "connection check and directory creation failed",
                );
            }
            self.checkpoint(run_id, attempt, log).await?;
        }
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

    async fn apply_retention(&self, plan: &Plan, log: &mut LogBuffer) {
        let archive_prefix = archive_prefix(plan);
        let include = format!("/{archive_prefix}*.{}", plan.archive.kind);
        for remote in &plan.remotes {
            let destination = remote_path(remote);
            if plan.retention.keep_days > 0 {
                log.line(format!(
                    "rclone delete {destination} --min-age {}d",
                    plan.retention.keep_days
                ));
                if let Err(error) = self
                    .rc
                    .run_command(
                        "delete",
                        vec![
                            destination.clone(),
                            "--min-age".into(),
                            format!("{}d", plan.retention.keep_days),
                            "--include".into(),
                            include.clone(),
                        ],
                        plan.rclone_flags.clone(),
                    )
                    .await
                {
                    log.line(format!("Age retention warning: {error}"));
                }
            }
            if plan.retention.keep_count > 0 {
                log.line(format!("rclone lsjson {destination} --files-only"));
                match self
                    .rc
                    .run_command_output(
                        "lsjson",
                        vec![destination.clone(), "--files-only".into()],
                        plan.rclone_flags.clone(),
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
                            let path = format!(
                                "{}/{}",
                                destination.trim_end_matches('/'),
                                item.path.trim_start_matches('/')
                            );
                            log.line(format!("rclone deletefile {path}"));
                            if let Err(error) = self
                                .rc
                                .run_command("deletefile", vec![path], plan.rclone_flags.clone())
                                .await
                            {
                                log.line(format!("Count retention warning: {error}"));
                            }
                        }
                    }
                    Err(error) => log.line(format!("Count retention warning: {error}")),
                }
            }
        }
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
    log: &mut LogBuffer,
) -> anyhow::Result<()> {
    fs::create_dir_all(staging).await?;
    if sources.len() == 1 {
        run_command(
            CommandSpec::new("cp")
                .args(["-a", "--"])
                .arg(format!("{}/.", sources[0].path.trim_end_matches('/')))
                .arg(staging.as_os_str()),
            log,
        )
        .await?;
    } else {
        for source in sources {
            let destination = staging.join(safe_name(&source.name));
            fs::create_dir_all(&destination).await?;
            run_command(
                CommandSpec::new("cp")
                    .args(["-a", "--"])
                    .arg(format!("{}/.", source.path.trim_end_matches('/')))
                    .arg(destination.as_os_str()),
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

async fn run_command(spec: CommandSpec, log: &mut LogBuffer) -> anyhow::Result<()> {
    capture_command(spec, log).await.map(|_| ())
}

async fn capture_command(spec: CommandSpec, log: &mut LogBuffer) -> anyhow::Result<String> {
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
    let output = child.wait_with_output().await?;
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

async fn send_notification(
    plan_name: &str,
    notifications: &NotificationConfig,
    event: &str,
    content: &str,
    log: &mut LogBuffer,
) {
    let report =
        rclone_backup_notifications::deliver(plan_name, notifications, event, content).await;
    for message in report.messages {
        log.line(message);
    }
}

async fn notify(
    plan: &Plan,
    notifications: &NotificationConfig,
    event: &str,
    content: &str,
    log: &mut LogBuffer,
) {
    send_notification(&plan.name, notifications, event, content, log).await;
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
        self.event(serde_json::json!({ "kind": "phase", "phase": phase }));
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
    fn names_and_secrets_are_safe() {
        assert_eq!(safe_name("My Files / 1"), "my-files---1");
        assert_eq!(redact("token=secret", &["secret".into()]), "token=••••••••");
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
        stage_sources(&refs, &staging, &mut log).await.unwrap();
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
        stage_sources(&refs, &staging, &mut log).await.unwrap();
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
