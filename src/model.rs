use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const REDACTED: &str = "••••••••";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub schedule: String,
    pub timezone: String,
    pub sources: Vec<FolderSource>,
    pub archive: ArchiveConfig,
    pub remotes: Vec<RemoteConfig>,
    pub retention: RetentionPolicy,
    pub retry: RetryPolicy,
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub rclone_flags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanInput {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub sources: Vec<FolderSource>,
    #[serde(default)]
    pub archive: ArchiveConfig,
    #[serde(default)]
    pub remotes: Vec<RemoteConfig>,
    #[serde(default)]
    pub retention: RetentionPolicy,
    #[serde(default)]
    pub retry: RetryPolicy,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub rclone_flags: Vec<String>,
}

impl PlanInput {
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err("name must contain 1 to 80 characters".into());
        }
        crate::schedule::parse_schedule(&self.schedule)?;
        self.timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| "timezone must be a valid IANA timezone".to_owned())?;
        if self.sources.is_empty() {
            return Err("at least one folder source is required".into());
        }
        if self.remotes.is_empty() {
            return Err("at least one rclone destination is required".into());
        }
        for source in &self.sources {
            validate_label(&source.name, "source name")?;
            if !source.path.starts_with('/') || source.path.contains('\0') {
                return Err("source paths must be absolute".into());
            }
        }
        let mut archive_names = std::collections::HashSet::new();
        if self.sources.iter().any(|source| {
            let name = crate::runner::safe_name(&source.name);
            name.is_empty() || !archive_names.insert(name)
        }) {
            return Err("source names must produce unique archive folder names".into());
        }
        for remote in &self.remotes {
            validate_label(&remote.name, "remote name")?;
            if remote.directory.contains('\0') || remote.directory.contains('\n') {
                return Err("remote directory contains invalid characters".into());
            }
        }
        if !matches!(self.archive.kind.as_str(), "zip" | "7z" | "none") {
            return Err("archive kind must be zip, 7z, or none".into());
        }
        if self.archive.suffix.is_empty()
            || self.archive.suffix.contains(['/', '\\', '\0', '\n'])
            || self.archive.suffix.chars().count() > 80
        {
            return Err("archive suffix is invalid".into());
        }
        let mut source_names = std::collections::HashSet::new();
        if self
            .sources
            .iter()
            .any(|source| !source_names.insert(source.name.trim().to_ascii_lowercase()))
        {
            return Err("source names must be unique".into());
        }
        if self.retry.max_attempts == 0 || self.retry.max_attempts > 20 {
            return Err("retry max_attempts must be between 1 and 20".into());
        }
        if !matches!(self.retry.backoff.as_str(), "fixed" | "exponential") {
            return Err("retry backoff must be fixed or exponential".into());
        }
        if self.retry.initial_delay_seconds > 86_400 || self.retry.max_delay_seconds > 86_400 {
            return Err("retry delays cannot exceed 86400 seconds".into());
        }
        if self.retention.keep_days > 36_500 || self.retention.keep_count > 100_000 {
            return Err("retention value is too large".into());
        }
        if self.archive.kind == "none"
            && (self.retention.keep_days > 0 || self.retention.keep_count > 0)
        {
            return Err("retention by archive age or count requires ZIP or 7z mode".into());
        }
        if self.rclone_flags.len() > 64 || self.rclone_flags.iter().any(|v| v.contains('\0')) {
            return Err("invalid rclone flags".into());
        }
        const BLOCKED_FLAGS: &[&str] = &[
            "--config",
            "--password-command",
            "--log-file",
            "--dump",
            "--dump-bodies",
            "--dump-headers",
            "--rc",
            "--rc-addr",
            "--rc-pass",
            "--rc-user",
            "--rc-web-gui",
        ];
        if self.rclone_flags.iter().any(|value| {
            BLOCKED_FLAGS.iter().any(|flag| {
                value == flag
                    || value
                        .strip_prefix(flag)
                        .is_some_and(|rest| rest.starts_with('='))
            })
        }) {
            return Err(
                "rclone flags contain a server, config, log, dump, or command option".into(),
            );
        }
        Ok(())
    }

    pub fn into_plan(self, id: Uuid, created_at: DateTime<Utc>) -> Plan {
        Plan {
            id,
            name: self.name.trim().to_owned(),
            enabled: self.enabled,
            schedule: self.schedule.trim().to_owned(),
            timezone: self.timezone.trim().to_owned(),
            sources: self.sources,
            archive: self.archive,
            remotes: self.remotes,
            retention: self.retention,
            retry: self.retry,
            notifications: self.notifications,
            rclone_flags: self.rclone_flags,
            created_at,
            updated_at: Utc::now(),
        }
    }
}

fn validate_label(value: &str, field: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || matches!(trimmed, "." | "..")
        || trimmed.chars().count() > 80
        || trimmed.contains(['/', ':', '\0', '\n'])
    {
        return Err(format!("{field} is invalid"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FolderSource {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchiveConfig {
    pub kind: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_suffix")]
    pub suffix: String,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            kind: "7z".into(),
            password: String::new(),
            suffix: default_suffix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteConfig {
    pub name: String,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RetentionPolicy {
    #[serde(default)]
    pub keep_days: u32,
    #[serde(default)]
    pub keep_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_delay")]
    pub initial_delay_seconds: u64,
    #[serde(default = "default_max_delay")]
    pub max_delay_seconds: u64,
    #[serde(default = "default_backoff")]
    pub backoff: String,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_seconds: 10,
            max_delay_seconds: 300,
            backoff: "exponential".into(),
        }
    }
}

impl RetryPolicy {
    pub fn delay_for(&self, failed_attempt: u32) -> u64 {
        if self.backoff == "fixed" {
            return self.initial_delay_seconds.min(self.max_delay_seconds);
        }
        let multiplier = 1_u64
            .checked_shl(failed_attempt.saturating_sub(1))
            .unwrap_or(u64::MAX);
        self.initial_delay_seconds
            .saturating_mul(multiplier)
            .min(self.max_delay_seconds)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NotificationConfig {
    #[serde(default)]
    pub ping: PingConfig,
    #[serde(default)]
    pub mail: MailConfig,
    #[serde(default)]
    pub serverchan: ServerChanConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PingConfig {
    #[serde(default)]
    pub completion_url: String,
    #[serde(default)]
    pub completion_options: Vec<String>,
    #[serde(default)]
    pub start_url: String,
    #[serde(default)]
    pub start_options: Vec<String>,
    #[serde(default)]
    pub success_url: String,
    #[serde(default)]
    pub success_options: Vec<String>,
    #[serde(default)]
    pub failure_url: String,
    #[serde(default)]
    pub failure_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub smtp_options: Vec<String>,
    #[serde(default)]
    pub to: String,
    #[serde(default = "default_true")]
    pub on_success: bool,
    #[serde(default = "default_true")]
    pub on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerChanConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub send_key: String,
    #[serde(default = "default_true")]
    pub on_start: bool,
    #[serde(default = "default_true")]
    pub on_success: bool,
    #[serde(default = "default_true")]
    pub on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub plan_id: String,
    pub plan_name: String,
    pub trigger: String,
    pub status: String,
    pub attempt: i64,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub log: String,
}

fn default_true() -> bool {
    true
}
fn default_schedule() -> String {
    "5 * * * *".into()
}
fn default_timezone() -> String {
    "UTC".into()
}
fn default_suffix() -> String {
    "%Y%m%d-%H%M%S".into()
}
fn default_attempts() -> u32 {
    3
}
fn default_delay() -> u64 {
    10
}
fn default_max_delay() -> u64 {
    300
}
fn default_backoff() -> String {
    "exponential".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_retry_is_capped() {
        let retry = RetryPolicy {
            max_attempts: 10,
            initial_delay_seconds: 5,
            max_delay_seconds: 20,
            backoff: "exponential".into(),
        };
        assert_eq!(
            [
                retry.delay_for(1),
                retry.delay_for(2),
                retry.delay_for(3),
                retry.delay_for(9)
            ],
            [5, 10, 20, 20]
        );
    }
}
