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
use std::{
    collections::{HashMap, HashSet},
    env,
    time::Duration,
};
use tokio::{net::TcpListener, process::Command};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const MAX_SCHEDULED_BACKUPS: usize = 2;

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
        Some("serverchan") => notification_test(config, "serverchan", args.get(1)).await,
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
            site_name: config.site_name,
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
        let mut activity = SchedulerActivity::default();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if !runner.rclone_ready() {
                continue;
            }
            let now = Utc::now();
            match store.list_plans().await {
                Ok(plans) => {
                    for plan in plans.into_iter().filter(|plan| plan.enabled) {
                        if !activity.should_evaluate(plan.id, runner.is_active(plan.id).await) {
                            last_slots.insert(plan.id, now);
                            continue;
                        }
                        let last = last_slots.get(&plan.id).copied();
                        if is_due_in_timezone(&plan.schedule, &plan.timezone, now, last) {
                            last_slots.insert(plan.id, now);
                            if let Err(error) = runner
                                .clone()
                                .start_scheduled(plan, "schedule", MAX_SCHEDULED_BACKUPS)
                                .await
                            {
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

#[derive(Default)]
struct SchedulerActivity {
    active_last_tick: HashSet<Uuid>,
}

impl SchedulerActivity {
    fn should_evaluate(&mut self, plan_id: Uuid, is_active: bool) -> bool {
        if is_active {
            self.active_last_tick.insert(plan_id);
            return false;
        }
        !self.active_last_tick.remove(&plan_id)
    }
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
    let (store, runner) = prepare(&config).await?;
    let notifications = store
        .confirmed_notifications()
        .await?
        .context("global notifications are not configured or confirmed")?;
    if argument.is_some() {
        tracing::warn!("notification test arguments are ignored; configure the global module");
    }
    runner.test_notification(&notifications, kind).await?;
    println!("{kind} sent successfully");
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
        "rclone-backup {}\n\nUsage:\n  rclone-backup [serve]\n  rclone-backup backup [PLAN_ID]\n  rclone-backup ping\n  rclone-backup mail\n  rclone-backup serverchan\n  rclone-backup <rclone|7z|curl|mail> [args...]\n\nNotification tests use the confirmed global configuration. The default command starts the Web UI.",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use super::SchedulerActivity;
    use uuid::Uuid;

    #[test]
    fn scheduler_consumes_slots_across_an_active_to_idle_transition() {
        let plan_id = Uuid::new_v4();
        let mut activity = SchedulerActivity::default();

        assert!(activity.should_evaluate(plan_id, false));
        assert!(!activity.should_evaluate(plan_id, true));
        assert!(!activity.should_evaluate(plan_id, false));
        assert!(activity.should_evaluate(plan_id, false));
    }
}
