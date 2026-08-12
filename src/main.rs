use anyhow::{Context, bail};
use chrono::Utc;
use rclone_backup::{
    api::{AppState, router},
    config::{AppConfig, plans_from_environment},
    rc::RcloneRc,
    runner::Runner,
    schedule::is_due_in_timezone,
    store::Store,
};
use std::{collections::HashMap, env, process::Stdio, time::Duration};
use tokio::{net::TcpListener, process::Command};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(&config.log_level)?)
        .init();
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--help" | "-h" | "help") => {
            print_help();
            Ok(())
        }
        Some("--version" | "-V" | "version") => {
            println!("rclone-backup {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("serve") | None => serve(config).await,
        Some("backup") => backup(config, args.get(1)).await,
        Some("ping") => notification_test(config, "ping", args.get(1)).await,
        Some("mail") => notification_test(config, "mail", args.get(1)).await,
        Some(_) => passthrough(&args, &config.rclone_config).await,
    }
}

async fn prepare(config: &AppConfig) -> anyhow::Result<(Store, Runner)> {
    if let Some(path) = config
        .database_url
        .strip_prefix("sqlite://")
        .and_then(|value| value.split('?').next())
        && let Some(parent) = std::path::Path::new(path).parent()
    {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::create_dir_all(&config.work_dir).await?;
    if let Some(parent) = std::path::Path::new(&config.rclone_config).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let store = Store::connect(
        &config.database_url,
        &config.secret_key_file,
        config.secret_key.as_deref(),
    )
    .await?;
    let plans = plans_from_environment().map_err(anyhow::Error::msg)?;
    if store.seed_once(&plans).await? {
        info!(plans = plans.len(), "imported environment configuration");
    }
    let rc = RcloneRc::start(&config.rclone_config).await?;
    Ok((store.clone(), Runner::new(store, &config.work_dir, rc)))
}

async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let (store, runner) = prepare(&config).await?;
    spawn_readiness_probe(runner.clone());
    spawn_scheduler(store.clone(), runner.clone());
    let listener = TcpListener::bind(&config.address)
        .await
        .with_context(|| format!("bind {}", config.address))?;
    info!(address = %config.address, version = env!("CARGO_PKG_VERSION"), "Web UI ready");
    if config.public_auth.is_none() {
        warn!(
            "Web authentication is disabled; do not expose this listener to an untrusted network"
        );
    }
    axum::serve(
        listener,
        router(AppState {
            store,
            runner,
            public_auth: config.public_auth,
        }),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

async fn backup(config: AppConfig, id: Option<&String>) -> anyhow::Result<()> {
    let (store, runner) = prepare(&config).await?;
    let plans = store.list_plans().await?;
    let plan = match id {
        Some(id) => store
            .get_plan(Uuid::parse_str(id)?)
            .await?
            .context("plan not found")?,
        None if plans.len() == 1 => plans.into_iter().next().unwrap(),
        None => plans
            .into_iter()
            .find(|plan| plan.enabled)
            .context("multiple plans exist; pass a plan ID")?,
    };
    runner.execute_sync(plan, "cli").await
}

fn spawn_scheduler(store: Store, runner: Runner) {
    tokio::spawn(async move {
        let mut last_slots = HashMap::new();
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if !runner.rclone_ready() {
                continue;
            }
            let now = Utc::now();
            match store.list_plans().await {
                Ok(plans) => {
                    for plan in plans.into_iter().filter(|plan| plan.enabled) {
                        let last = last_slots.get(&plan.id).copied();
                        if is_due_in_timezone(&plan.schedule, &plan.timezone, now, last) {
                            last_slots.insert(plan.id, now);
                            if let Err(error) = runner.clone().start(plan, "schedule").await {
                                warn!(%error, "scheduled plan skipped");
                            }
                        }
                    }
                }
                Err(error) => warn!(%error, "scheduler cannot load plans"),
            }
        }
    });
}

fn spawn_readiness_probe(runner: Runner) {
    tokio::spawn(async move {
        let mut last = None;
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let ready = runner.rc_refresh_ready().await;
            if last != Some(ready) {
                if ready {
                    info!("rclone configuration detected; backup scheduling is ready");
                } else {
                    warn!("waiting for at least one rclone remote; Web UI stays available");
                }
                last = Some(ready);
            }
        }
    });
}

async fn notification_test(
    config: AppConfig,
    kind: &str,
    argument: Option<&String>,
) -> anyhow::Result<()> {
    let (store, _) = prepare(&config).await?;
    let mut plans = store.list_plans().await?;
    let plan = plans.pop().context("no backup plan")?;
    match kind {
        "ping" => {
            let event = argument.map(String::as_str).unwrap_or("success");
            let url = match event {
                "completion" => &plan.notifications.ping.completion_url,
                "start" => &plan.notifications.ping.start_url,
                "success" => &plan.notifications.ping.success_url,
                "failure" => &plan.notifications.ping.failure_url,
                _ => bail!("ping identifier must be completion, start, success, or failure"),
            };
            if url.is_empty() {
                bail!("ping URL is not configured");
            }
            let options = match event {
                "completion" => &plan.notifications.ping.completion_options,
                "start" => &plan.notifications.ping.start_options,
                "success" => &plan.notifications.ping.success_options,
                "failure" => &plan.notifications.ping.failure_options,
                _ => unreachable!(),
            };
            let mut command = Command::new("curl");
            command.args(["-f", "-m", "15", "--retry", "3"]);
            command.args(options);
            let status = command
                .arg(
                    url.replace("%{subject}", "RcloneBackup+Test")
                        .replace("%{content}", "Notification+test"),
                )
                .status()
                .await?;
            if !status.success() {
                bail!("ping command failed");
            }
            println!("ping sent successfully");
        }
        "mail" => {
            let recipient = argument.cloned().unwrap_or(plan.notifications.mail.to);
            if recipient.is_empty() {
                bail!("mail recipient is not configured");
            }
            let mut child = Command::new("mail")
                .args(["-s", "RcloneBackup Test", &recipient])
                .args(&plan.notifications.mail.smtp_options)
                .stdin(Stdio::piped())
                .spawn()?;
            use tokio::io::AsyncWriteExt;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(b"RcloneBackup notification test\n")
                .await?;
            if !child.wait().await?.success() {
                bail!("mail command failed");
            }
            println!("mail sent successfully");
        }
        _ => unreachable!(),
    }
    Ok(())
}

async fn passthrough(args: &[String], rclone_config: &str) -> anyhow::Result<()> {
    const ALLOWED_COMMANDS: &[&str] = &["rclone", "7z", "curl", "mail"];
    if !ALLOWED_COMMANDS.contains(&args[0].as_str()) {
        bail!("unsupported command; use rclone, 7z, curl, or mail");
    }
    let mut command = Command::new(&args[0]);
    if args[0] == "rclone" {
        command.args(["--config", rclone_config]);
    }
    let status = command
        .args(&args[1..])
        .status()
        .await
        .with_context(|| format!("start {}", args[0]))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}

fn print_help() {
    println!(
        "rclone-backup {}\n\nUsage:\n  rclone-backup [serve]\n  rclone-backup backup [PLAN_ID]\n  rclone-backup ping [completion|start|success|failure]\n  rclone-backup mail [RECIPIENT]\n  rclone-backup <rclone|7z|curl|mail> [args...]\n\nThe default command starts the Web UI.",
        env!("CARGO_PKG_VERSION")
    );
}
