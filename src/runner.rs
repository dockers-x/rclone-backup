use crate::{model::*, rc::RcloneRc, store::Store};
use anyhow::{Context, anyhow, bail};
use chrono::Local;
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
    sync::Mutex,
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

    pub async fn start(self, plan: Plan, trigger: &str) -> anyhow::Result<String> {
        if !self.rc.is_ready() {
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
        let run_id = run.id.clone();
        tokio::spawn(async move {
            if let Err(error) = self.execute(plan, run).await {
                warn!(%error, "backup run failed");
            }
        });
        Ok(run_id)
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
        let mut log = LogBuffer::new(plan);
        let mut result = Err(anyhow!("backup did not run"));
        let mut final_attempt = 1;
        for attempt in 1..=plan.retry.max_attempts {
            final_attempt = attempt;
            log.line(format!("Attempt {attempt}/{}", plan.retry.max_attempts));
            self.store
                .update_run(&run.id, "running", attempt, &log.text, false)
                .await?;
            result = self.backup_once(plan, &mut log).await;
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
            log.line(format!("Backup failed: {error:#}"));
            notify(
                plan,
                "failure",
                &format!("Backup failed at {}. Reason: {error:#}", Local::now()),
                &mut log,
            )
            .await;
        }
        self.store
            .update_run(&run.id, status, final_attempt, &log.text, true)
            .await?;
        result
    }

    async fn backup_once(&self, plan: &Plan, log: &mut LogBuffer) -> anyhow::Result<()> {
        let run_dir = self
            .work_dir
            .join(format!("{}-{}", safe_name(&plan.name), Uuid::new_v4()));
        fs::create_dir_all(&run_dir)
            .await
            .context("create working directory")?;
        let result = self.backup_in_dir(plan, &run_dir, log).await;
        if let Err(error) = fs::remove_dir_all(&run_dir).await {
            warn!(%error, "cleanup working directory");
        }
        result
    }

    async fn backup_in_dir(
        &self,
        plan: &Plan,
        run_dir: &Path,
        log: &mut LogBuffer,
    ) -> anyhow::Result<()> {
        notify(
            plan,
            "start",
            &format!("Start backup at {}", Local::now()),
            log,
        )
        .await;
        self.ensure_workspace_outside_sources(plan, run_dir)?;
        self.check_remotes(plan, log).await?;
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
        for remote in &plan.remotes {
            let destination = if plan.archive.kind == "none" {
                format!(
                    "{}/{archive_base}",
                    remote_path(remote).trim_end_matches('/')
                )
            } else {
                remote_path(remote)
            };
            log.line(format!("rclone copy {} {destination}", upload.display()));
            self.rc
                .run_command(
                    "copy",
                    vec![upload.to_string_lossy().into_owned(), destination.clone()],
                    plan.rclone_flags.clone(),
                )
                .await
                .with_context(|| format!("upload to {destination}"))?;
        }
        self.apply_retention(plan, log).await;
        notify(
            plan,
            "success",
            &format!("Backup completed at {}", Local::now()),
            log,
        )
        .await;
        Ok(())
    }

    async fn check_remotes(&self, plan: &Plan, log: &mut LogBuffer) -> anyhow::Result<()> {
        let mut available = 0;
        for remote in &plan.remotes {
            let destination = remote_path(remote);
            log.line(format!("rclone lsd {destination}"));
            if self
                .rc
                .run_command("lsd", vec![destination.clone()], plan.rclone_flags.clone())
                .await
                .is_ok()
            {
                available += 1;
                continue;
            }
            log.line(format!("rclone mkdir {destination}"));
            if self
                .rc
                .run_command("mkdir", vec![destination], plan.rclone_flags.clone())
                .await
                .is_ok()
            {
                available += 1;
            }
        }
        if available == 0 {
            bail!("all rclone destinations are unavailable");
        }
        Ok(())
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

async fn notify(plan: &Plan, event: &str, content: &str, log: &mut LogBuffer) {
    let subject = format!(
        "{} Backup {}",
        plan.name,
        match event {
            "start" => "Start",
            "success" => "Success",
            _ => "Failed",
        }
    );
    let ping = &plan.notifications.ping;
    let mut endpoints = vec![];
    if event == "start" {
        endpoints.push((&ping.start_url, &ping.start_options));
    }
    if event == "success" {
        endpoints.push((&ping.success_url, &ping.success_options));
    }
    if event == "failure" {
        endpoints.push((&ping.failure_url, &ping.failure_options));
    }
    if event != "start" {
        endpoints.push((&ping.completion_url, &ping.completion_options));
    }
    for (url, options) in endpoints {
        if url.is_empty() {
            continue;
        }
        let url = url
            .replace("%{subject}", &urlencoding(&subject))
            .replace("%{content}", &urlencoding(content));
        let args: Vec<_> = options
            .iter()
            .map(|v| {
                v.replace("%{subject}", &subject)
                    .replace("%{content}", content)
            })
            .collect();
        let mut command = CommandSpec::new("curl").args([
            "-m",
            "15",
            "--retry",
            "3",
            "--retry-delay",
            "1",
            "-o",
            "/dev/null",
            "-s",
        ]);
        command.args.extend(args);
        command = command.arg(&url).secret(&url);
        if let Err(error) = run_command(command, log).await {
            log.line(format!("Ping notification warning: {error}"));
        }
    }
    let mail = &plan.notifications.mail;
    if mail.enabled
        && !mail.to.is_empty()
        && ((event == "success" && mail.on_success) || (event == "failure" && mail.on_failure))
    {
        let mut command = CommandSpec::new("mail").args(["-s", &subject]);
        command.args.extend(mail.smtp_options.clone());
        command = command.arg(&mail.to);
        let child = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        match child {
            Ok(mut child) => {
                if let Some(stdin) = &mut child.stdin {
                    let _ = stdin.write_all(content.as_bytes()).await;
                }
                drop(child.stdin.take());
                match child.wait_with_output().await {
                    Ok(out) if out.status.success() => log.line("Mail notification sent."),
                    Ok(out) => log.line(format!(
                        "Mail notification warning: {}",
                        String::from_utf8_lossy(&out.stderr)
                    )),
                    Err(e) => log.line(format!("Mail notification warning: {e}")),
                }
            }
            Err(e) => log.line(format!("Mail notification warning: {e}")),
        }
    }
    let server = &plan.notifications.serverchan;
    if server.enabled
        && !server.send_key.is_empty()
        && ((event == "start" && server.on_start)
            || (event == "success" && server.on_success)
            || (event == "failure" && server.on_failure))
    {
        let url = if let Some(rest) = server.send_key.strip_prefix("sctp") {
            let number: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            format!(
                "https://{number}.push.ft07.com/send/{}.send",
                server.send_key
            )
        } else {
            format!("https://sctapi.ftqq.com/{}.send", server.send_key)
        };
        let command = CommandSpec::new("curl")
            .args([
                "-m",
                "15",
                "--retry",
                "3",
                "-s",
                "-o",
                "/dev/null",
                "-X",
                "POST",
                "--data-urlencode",
                &format!("text={subject}"),
                "--data-urlencode",
                &format!("desp={content}"),
            ])
            .arg(&url)
            .secret(&server.send_key);
        if let Err(error) = run_command(command, log).await {
            log.line(format!("ServerChan warning: {error}"));
        }
    }
}

pub(crate) fn safe_name(value: &str) -> String {
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
fn urlencoding(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

struct LogBuffer {
    text: String,
    secrets: Vec<String>,
}
impl LogBuffer {
    fn new(plan: &Plan) -> Self {
        let secrets = vec![
            plan.archive.password.clone(),
            plan.notifications.serverchan.send_key.clone(),
        ];
        Self {
            text: String::new(),
            secrets,
        }
    }
    fn line(&mut self, value: impl AsRef<str>) {
        let line = redact(value.as_ref(), &self.secrets);
        if self.text.len() < MAX_LOG_BYTES {
            self.text.push_str(&format!(
                "[{}] {}\n",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                line
            ));
            self.text
                .truncate(self.text.floor_char_boundary(MAX_LOG_BYTES));
        }
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
