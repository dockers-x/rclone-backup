use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use uuid::Uuid;

pub const REDACTED: &str = "••••••••";
pub const DEFAULT_REMOTE_CHECK_CONCURRENCY: usize = 4;
pub const MAX_REMOTE_CHECK_CONCURRENCY: usize = 32;
pub const DEFAULT_UPLOAD_CONCURRENCY: usize = 1;
pub const MAX_UPLOAD_CONCURRENCY: usize = 8;

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
    #[serde(default, skip_serializing_if = "NotificationConfig::is_empty")]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub rclone_flags: Vec<String>,
    #[serde(default = "default_remote_check_concurrency")]
    pub remote_check_concurrency: usize,
    #[serde(default = "default_upload_concurrency")]
    pub upload_concurrency: usize,
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
    #[serde(default, skip_serializing_if = "NotificationConfig::is_empty")]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub rclone_flags: Vec<String>,
    #[serde(default = "default_remote_check_concurrency")]
    pub remote_check_concurrency: usize,
    #[serde(default = "default_upload_concurrency")]
    pub upload_concurrency: usize,
}

impl PlanInput {
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.contains(['\0', '\r', '\n']) {
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
            let name = safe_archive_name(&source.name);
            name.is_empty() || !archive_names.insert(name)
        }) {
            return Err("source names must produce unique archive folder names".into());
        }
        let mut remote_destinations = std::collections::HashSet::new();
        for remote in &self.remotes {
            validate_label(&remote.name, "remote name")?;
            if remote.directory.contains('\0') || remote.directory.contains('\n') {
                return Err("remote directory contains invalid characters".into());
            }
            if remote.directory.contains("//")
                || remote
                    .directory
                    .split('/')
                    .any(|segment| matches!(segment, "." | ".."))
            {
                return Err(
                    "remote directory cannot contain dot segments or repeated separators".into(),
                );
            }
            let destination = format!(
                "{}:{}",
                remote.name.trim(),
                remote.directory.trim_end_matches('/')
            );
            if !remote_destinations.insert(destination) {
                return Err("rclone destinations must be unique".into());
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
        if self.archive.password_hint.chars().count() > 160
            || self.archive.password_hint.contains(['\0', '\r', '\n'])
        {
            return Err("archive password hint is invalid".into());
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
        if !(1..=MAX_REMOTE_CHECK_CONCURRENCY).contains(&self.remote_check_concurrency) {
            return Err(format!(
                "remote check concurrency must be between 1 and {MAX_REMOTE_CHECK_CONCURRENCY}"
            ));
        }
        if !(1..=MAX_UPLOAD_CONCURRENCY).contains(&self.upload_concurrency) {
            return Err(format!(
                "upload concurrency must be between 1 and {MAX_UPLOAD_CONCURRENCY}"
            ));
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
            schedule: self.schedule,
            timezone: self.timezone.trim().to_owned(),
            sources: self.sources,
            archive: self.archive,
            remotes: self.remotes,
            retention: self.retention,
            retry: self.retry,
            notifications: self.notifications,
            rclone_flags: self.rclone_flags,
            remote_check_concurrency: self.remote_check_concurrency,
            upload_concurrency: self.upload_concurrency,
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

fn safe_archive_name(value: &str) -> String {
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
    #[serde(default)]
    pub password_hint: String,
    #[serde(default = "default_suffix")]
    pub suffix: String,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            kind: "7z".into(),
            password: String::new(),
            password_hint: String::new(),
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<NotificationTarget>,
    #[serde(default, skip_serializing_if = "ping_is_default")]
    pub ping: PingConfig,
    #[serde(default, skip_serializing_if = "mail_is_default")]
    pub mail: MailConfig,
    #[serde(default, skip_serializing_if = "serverchan_is_default")]
    pub serverchan: ServerChanConfig,
}

impl NotificationConfig {
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
            && !self.ping.has_endpoint()
            && !self.mail.enabled
            && self.mail.to.trim().is_empty()
            && self.mail.smtp_options.is_empty()
            && !self.serverchan.enabled
            && self.serverchan.send_key.trim().is_empty()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.targets.len() > 32 {
            return Err("at most 32 notification targets are allowed".into());
        }
        let mut ids = std::collections::HashSet::new();
        for target in &self.targets {
            target.validate()?;
            if !ids.insert(target.id.as_str()) {
                return Err("notification target IDs must be unique".into());
            }
        }
        self.ping.validate()?;
        self.validate_mail()?;
        self.validate_serverchan()?;
        Ok(())
    }

    fn validate_mail(&self) -> Result<(), String> {
        if self.mail.to.chars().count() > 320 || self.mail.smtp_options.len() > 64 {
            return Err("SMTP notification configuration is too large".into());
        }
        if self.mail.enabled && self.mail.to.trim().is_empty() {
            return Err("SMTP recipient is required when SMTP is enabled".into());
        }
        if self.mail.enabled && smtp_server_from_options(&self.mail.smtp_options).is_none() {
            return Err("SMTP server is required when SMTP is enabled".into());
        }
        if !self.mail.to.is_empty() {
            validate_mail_recipient(&self.mail.to)?;
        }
        if self.mail.enabled
            && !(self.mail.on_start || self.mail.on_success || self.mail.on_failure)
        {
            return Err("at least one SMTP event is required when SMTP is enabled".into());
        }
        validate_mail_options(&self.mail.smtp_options)?;
        Ok(())
    }

    fn validate_serverchan(&self) -> Result<(), String> {
        if !matches!(self.serverchan.channel.as_str(), "app" | "wechat") {
            return Err("ServerChan channel must be app or wechat".into());
        }
        if self.serverchan.send_key.chars().count() > 256
            || self.serverchan.send_key.contains(['\0', '\r', '\n'])
        {
            return Err("ServerChan SendKey is invalid".into());
        }
        if self.serverchan.enabled && self.serverchan.send_key.trim().is_empty() {
            return Err("ServerChan SendKey is required when ServerChan is enabled".into());
        }
        if self.serverchan.enabled
            && !(self.serverchan.on_start
                || self.serverchan.on_success
                || self.serverchan.on_failure)
        {
            return Err(
                "at least one ServerChan event is required when ServerChan is enabled".into(),
            );
        }
        Ok(())
    }

    pub async fn isolate_migration_channels(mut self) -> (Self, Vec<NotificationMigrationWarning>) {
        let mut config = Self::default();
        let mut warnings = Vec::new();
        let mail_normalization = self.normalize_legacy_mail();

        match self.ping.validate() {
            Ok(()) => match validate_ping_network_targets(&self.ping).await {
                Ok(()) => config.ping = self.ping.clone(),
                Err(reason) => warnings.push(NotificationMigrationWarning::new("ping", reason)),
            },
            Err(reason) => warnings.push(NotificationMigrationWarning::new("ping", reason)),
        }

        let mail_result = mail_normalization.and_then(|_| self.validate_mail());
        match mail_result {
            Ok(()) => match validate_mail_network_target(&self.mail).await {
                Ok(()) => config.mail = self.mail.clone(),
                Err(reason) => warnings.push(NotificationMigrationWarning::new("mail", reason)),
            },
            Err(reason) => warnings.push(NotificationMigrationWarning::new("mail", reason)),
        }

        match self.validate_serverchan() {
            Ok(()) => config.serverchan = self.serverchan.clone(),
            Err(reason) => {
                warnings.push(NotificationMigrationWarning::new("serverchan", reason));
            }
        }

        (config, warnings)
    }

    pub fn normalize_legacy_mail(&mut self) -> Result<(), String> {
        self.mail.smtp_options = normalize_legacy_mail_options(&self.mail.smtp_options)?;
        Ok(())
    }

    pub fn normalize_legacy(&mut self) {
        if !self.ping.has_endpoint() {
            self.ping.enabled = false;
        } else if !self.ping.enabled {
            self.ping.enabled = true;
        }
    }

    pub fn promote_legacy_targets(&mut self) {
        if !self.targets.is_empty() {
            return;
        }
        if self.ping.has_endpoint() {
            self.targets.push(NotificationTarget {
                id: "legacy-ping".into(),
                name: "Ping".into(),
                enabled: self.ping.enabled,
                on_start: self.ping.on_start,
                on_success: self.ping.on_success,
                on_failure: self.ping.on_failure,
                kind: NotificationTargetKind::Ping {
                    config: PingTargetConfig {
                        completion_url: self.ping.completion_url.clone(),
                        completion_options: self.ping.completion_options.clone(),
                        start_url: self.ping.start_url.clone(),
                        start_options: self.ping.start_options.clone(),
                        success_url: self.ping.success_url.clone(),
                        success_options: self.ping.success_options.clone(),
                        failure_url: self.ping.failure_url.clone(),
                        failure_options: self.ping.failure_options.clone(),
                    },
                },
            });
        }
        if self.mail.enabled || !self.mail.to.is_empty() || !self.mail.smtp_options.is_empty() {
            self.targets.push(NotificationTarget {
                id: "legacy-mail".into(),
                name: "Email".into(),
                enabled: self.mail.enabled,
                on_start: self.mail.on_start,
                on_success: self.mail.on_success,
                on_failure: self.mail.on_failure,
                kind: NotificationTargetKind::Email {
                    config: MailTargetConfig::from_legacy(&self.mail),
                },
            });
        }
        if self.serverchan.enabled || !self.serverchan.send_key.is_empty() {
            let name = if self.serverchan.channel == "app" {
                "Server酱 App 推送"
            } else {
                "Server酱微信推送"
            };
            self.targets.push(NotificationTarget {
                id: "legacy-serverchan".into(),
                name: name.into(),
                enabled: self.serverchan.enabled,
                on_start: self.serverchan.on_start,
                on_success: self.serverchan.on_success,
                on_failure: self.serverchan.on_failure,
                kind: NotificationTargetKind::ServerChan {
                    config: ServerChanTargetConfig {
                        channel: self.serverchan.channel.parse().unwrap_or_default(),
                        send_key: self.serverchan.send_key.clone(),
                    },
                },
            });
        }
        if !self.targets.is_empty() {
            self.ping = Default::default();
            self.mail = Default::default();
            self.serverchan = Default::default();
        }
    }

    pub fn normalize_email_targets(&mut self) {
        for target in &mut self.targets {
            let NotificationTargetKind::Email { config } = &mut target.kind else {
                continue;
            };
            if config.host.is_empty() && !config.smtp_options.is_empty() {
                let legacy = MailConfig {
                    smtp_options: config.smtp_options.clone(),
                    to: config.to.clone(),
                    ..Default::default()
                };
                *config = MailTargetConfig::from_legacy(&legacy);
            }
        }
    }

    pub fn merge_redacted_from(&mut self, existing: &Self) {
        for target in &mut self.targets {
            if let Some(old) = existing
                .targets
                .iter()
                .find(|old| old.id == target.id && old.kind.same_variant(&target.kind))
            {
                target.merge_redacted_from(old);
            }
        }
        for (value, old) in [
            (&mut self.ping.completion_url, &existing.ping.completion_url),
            (&mut self.ping.start_url, &existing.ping.start_url),
            (&mut self.ping.success_url, &existing.ping.success_url),
            (&mut self.ping.failure_url, &existing.ping.failure_url),
        ] {
            if value == REDACTED {
                value.clone_from(old);
            }
        }
        if self.serverchan.send_key == REDACTED {
            self.serverchan
                .send_key
                .clone_from(&existing.serverchan.send_key);
        }
        if self.mail.smtp_options.as_slice() == [REDACTED] {
            self.mail
                .smtp_options
                .clone_from(&existing.mail.smtp_options);
        } else {
            for (index, value) in self.mail.smtp_options.iter_mut().enumerate() {
                if value == REDACTED || value.ends_with(REDACTED) {
                    let old = value
                        .split_once('=')
                        .and_then(|(key, _)| {
                            existing.mail.smtp_options.iter().find(|old| {
                                old.split_once('=')
                                    .is_some_and(|(old_key, _)| old_key == key)
                            })
                        })
                        .or_else(|| existing.mail.smtp_options.get(index));
                    if let Some(old) = old {
                        value.clone_from(old);
                    }
                }
            }
        }
        for (values, old_values) in [
            (
                &mut self.ping.completion_options,
                &existing.ping.completion_options,
            ),
            (&mut self.ping.start_options, &existing.ping.start_options),
            (
                &mut self.ping.success_options,
                &existing.ping.success_options,
            ),
            (
                &mut self.ping.failure_options,
                &existing.ping.failure_options,
            ),
        ] {
            if values.iter().all(|value| value == REDACTED) && !values.is_empty() {
                values.clone_from(old_values);
            }
        }
    }

    pub async fn validate_network_targets(&self) -> Result<(), String> {
        for target in &self.targets {
            target.validate_network_target().await?;
        }
        validate_ping_network_targets(&self.ping).await?;
        validate_mail_network_target(&self.mail).await?;
        Ok(())
    }
}

fn ping_is_default(value: &PingConfig) -> bool {
    value == &PingConfig::default()
}

fn mail_is_default(value: &MailConfig) -> bool {
    value == &MailConfig::default()
}

fn serverchan_is_default(value: &ServerChanConfig) -> bool {
    value == &ServerChanConfig::default()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationTarget {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub on_start: bool,
    #[serde(default = "default_true")]
    pub on_success: bool,
    #[serde(default = "default_true")]
    pub on_failure: bool,
    #[serde(flatten)]
    pub kind: NotificationTargetKind,
}

impl NotificationTarget {
    fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.chars().count() > 80
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("notification target ID is invalid".into());
        }
        validate_label(&self.name, "notification target name")?;
        if self.enabled && !(self.on_start || self.on_success || self.on_failure) {
            return Err("at least one notification event is required".into());
        }
        match &self.kind {
            NotificationTargetKind::Ping { config } => config.as_legacy(self).validate(),
            NotificationTargetKind::Email { config } => {
                let config = config.as_legacy(self);
                NotificationConfig {
                    mail: config,
                    ..Default::default()
                }
                .validate_mail()
            }
            NotificationTargetKind::ServerChan { config } => config.validate(self.enabled),
            NotificationTargetKind::Ntfy { config } => config.validate(self.enabled),
        }
    }

    async fn validate_network_target(&self) -> Result<(), String> {
        match &self.kind {
            NotificationTargetKind::Ping { config } => {
                validate_ping_network_targets(&config.as_legacy(self)).await
            }
            NotificationTargetKind::Email { config } => {
                validate_mail_network_target(&config.as_legacy(self)).await
            }
            NotificationTargetKind::Ntfy { config } => {
                validate_public_url(&config.server, "ntfy").await
            }
            NotificationTargetKind::ServerChan { .. } => Ok(()),
        }
    }

    fn merge_redacted_from(&mut self, old: &Self) {
        match (&mut self.kind, &old.kind) {
            (
                NotificationTargetKind::ServerChan { config },
                NotificationTargetKind::ServerChan { config: old },
            ) if config.send_key == REDACTED => config.send_key.clone_from(&old.send_key),
            (
                NotificationTargetKind::Email { config },
                NotificationTargetKind::Email { config: old },
            ) if config.password == REDACTED => config.password.clone_from(&old.password),
            (
                NotificationTargetKind::Ntfy { config },
                NotificationTargetKind::Ntfy { config: old },
            ) if config.token == REDACTED => config.token.clone_from(&old.token),
            (
                NotificationTargetKind::Ping { config },
                NotificationTargetKind::Ping { config: old },
            ) => {
                for (value, old_value) in [
                    (&mut config.completion_url, &old.completion_url),
                    (&mut config.start_url, &old.start_url),
                    (&mut config.success_url, &old.success_url),
                    (&mut config.failure_url, &old.failure_url),
                ] {
                    if value == REDACTED {
                        value.clone_from(old_value);
                    }
                }
                for (values, old_values) in [
                    (&mut config.completion_options, &old.completion_options),
                    (&mut config.start_options, &old.start_options),
                    (&mut config.success_options, &old.success_options),
                    (&mut config.failure_options, &old.failure_options),
                ] {
                    if values.as_slice() == [REDACTED] {
                        values.clone_from(old_values);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn as_notification_config(&self) -> NotificationConfig {
        let mut config = NotificationConfig::default();
        match &self.kind {
            NotificationTargetKind::Ping { config: value } => config.ping = value.as_legacy(self),
            NotificationTargetKind::Email { config: value } => config.mail = value.as_legacy(self),
            NotificationTargetKind::ServerChan { config: value } => {
                config.serverchan = ServerChanConfig {
                    enabled: self.enabled,
                    send_key: value.send_key.clone(),
                    channel: value.channel.to_string(),
                    on_start: self.on_start,
                    on_success: self.on_success,
                    on_failure: self.on_failure,
                };
            }
            NotificationTargetKind::Ntfy { .. } => {}
        }
        config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NotificationTargetKind {
    Ping {
        #[serde(default)]
        config: PingTargetConfig,
    },
    Email {
        #[serde(default)]
        config: MailTargetConfig,
    },
    #[serde(rename = "serverchan")]
    ServerChan {
        #[serde(default)]
        config: ServerChanTargetConfig,
    },
    Ntfy {
        #[serde(default)]
        config: NtfyTargetConfig,
    },
}

impl NotificationTargetKind {
    pub fn same_variant(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PingTargetConfig {
    pub completion_url: String,
    pub completion_options: Vec<String>,
    pub start_url: String,
    pub start_options: Vec<String>,
    pub success_url: String,
    pub success_options: Vec<String>,
    pub failure_url: String,
    pub failure_options: Vec<String>,
}

impl PingTargetConfig {
    fn as_legacy(&self, target: &NotificationTarget) -> PingConfig {
        PingConfig {
            enabled: target.enabled,
            on_start: target.on_start,
            on_success: target.on_success,
            on_failure: target.on_failure,
            completion_url: self.completion_url.clone(),
            completion_options: self.completion_options.clone(),
            start_url: self.start_url.clone(),
            start_options: self.start_options.clone(),
            success_url: self.success_url.clone(),
            success_options: self.success_options.clone(),
            failure_url: self.failure_url.clone(),
            failure_options: self.failure_options.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MailTargetConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_smtp_port")]
    pub port: u16,
    #[serde(default)]
    pub security: SmtpSecurity,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub smtp_options: Vec<String>,
    pub to: String,
}

impl MailTargetConfig {
    pub fn from_legacy(mail: &MailConfig) -> Self {
        let values = mail_option_values(&mail.smtp_options).unwrap_or_default();
        let server = values
            .get("mta")
            .or_else(|| values.get("smtp"))
            .and_then(|value| url::Url::parse(value).ok());
        let security = if server.as_ref().is_some_and(|url| url.scheme() == "smtps") {
            SmtpSecurity::Tls
        } else {
            SmtpSecurity::Starttls
        };
        Self {
            host: server
                .as_ref()
                .and_then(url::Url::host_str)
                .unwrap_or_default()
                .to_owned(),
            port: server
                .as_ref()
                .and_then(url::Url::port_or_known_default)
                .unwrap_or_else(|| {
                    if security == SmtpSecurity::Tls {
                        465
                    } else {
                        587
                    }
                }),
            security,
            from: values.get("from").cloned().unwrap_or_default(),
            username: values.get("smtp-auth-user").cloned().unwrap_or_default(),
            password: values
                .get("smtp-auth-password")
                .cloned()
                .unwrap_or_default(),
            smtp_options: Vec::new(),
            to: mail.to.clone(),
        }
    }

    fn as_legacy(&self, target: &NotificationTarget) -> MailConfig {
        let smtp_options = if self.host.is_empty() && !self.smtp_options.is_empty() {
            self.smtp_options.clone()
        } else {
            let mut options = Vec::new();
            let scheme = if self.security == SmtpSecurity::Tls {
                "smtps"
            } else {
                "smtp"
            };
            for value in [
                (!self.host.is_empty())
                    .then(|| format!("mta={scheme}://{}:{}", self.host, self.port)),
                (!self.from.is_empty()).then(|| format!("from={}", self.from)),
                (!self.username.is_empty()).then(|| format!("smtp-auth-user={}", self.username)),
                (!self.password.is_empty())
                    .then(|| format!("smtp-auth-password={}", self.password)),
            ]
            .into_iter()
            .flatten()
            {
                options.extend(["-S".to_owned(), value]);
            }
            options
        };
        MailConfig {
            enabled: target.enabled,
            smtp_options,
            to: self.to.clone(),
            on_start: target.on_start,
            on_success: target.on_success,
            on_failure: target.on_failure,
        }
    }
}

fn default_smtp_port() -> u16 {
    587
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SmtpSecurity {
    #[default]
    Starttls,
    Tls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerChanTargetConfig {
    #[serde(default)]
    pub channel: ServerChanChannel,
    pub send_key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServerChanChannel {
    App,
    #[default]
    Wechat,
}

impl std::fmt::Display for ServerChanChannel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::App => "app",
            Self::Wechat => "wechat",
        })
    }
}

impl std::str::FromStr for ServerChanChannel {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "app" => Ok(Self::App),
            "wechat" => Ok(Self::Wechat),
            _ => Err("ServerChan channel must be app or wechat".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NtfyTargetConfig {
    pub server: String,
    pub topic: String,
    pub token: String,
}

impl NtfyTargetConfig {
    fn validate(&self, enabled: bool) -> Result<(), String> {
        if self.server.chars().count() > 2048
            || self.server.contains(['\0', '\r', '\n'])
            || self.topic.is_empty()
            || self.topic.chars().count() > 64
            || !self
                .topic
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            || self.token.chars().count() > 1024
            || self.token.contains(['\0', '\r', '\n'])
        {
            return Err("ntfy notification configuration is invalid".into());
        }
        if enabled {
            let url = url::Url::parse(&self.server)
                .map_err(|_| "ntfy server URL is invalid".to_owned())?;
            if url.scheme() != "https" || url.host_str().is_none() {
                return Err("ntfy server URL must use HTTPS".into());
            }
        }
        Ok(())
    }
}

impl ServerChanTargetConfig {
    fn validate(&self, enabled: bool) -> Result<(), String> {
        if self.send_key.chars().count() > 256
            || self.send_key.contains(['\0', '\r', '\n'])
            || (enabled && self.send_key.trim().is_empty())
        {
            return Err("ServerChan SendKey is invalid".into());
        }
        Ok(())
    }
}

fn normalize_legacy_mail_options(options: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    let mut index = 0;
    while index < options.len() {
        let value = if options[index] == "-S" {
            index += 1;
            options
                .get(index)
                .map(String::as_str)
                .ok_or_else(|| "SMTP option is missing its value".to_owned())?
        } else {
            options[index]
                .strip_prefix("-S")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "SMTP options may only set supported SMTP variables".to_owned())?
        };

        let converted = match value {
            "v15-compat" | "smtp-use-starttls" => None,
            _ => {
                let (name, setting) = value
                    .split_once('=')
                    .ok_or_else(|| "SMTP variable must use name=value".to_owned())?;
                match name {
                    "smtp-auth" if setting.eq_ignore_ascii_case("login") => None,
                    "user" => Some(format!("smtp-auth-user={setting}")),
                    "password" => Some(format!("smtp-auth-password={setting}")),
                    "from" => Some(format!(
                        "from={}",
                        legacy_mail_address(setting).ok_or_else(|| {
                            "SMTP from address must be one email address".to_owned()
                        })?
                    )),
                    "mta" | "smtp" | "smtp-auth-user" | "smtp-auth-password" => {
                        Some(value.to_owned())
                    }
                    _ => return Err("SMTP variable is not allowed".into()),
                }
            }
        };
        if let Some(value) = converted {
            normalized.extend(["-S".to_owned(), value]);
        }
        index += 1;
    }
    validate_mail_options(&normalized)?;
    Ok(normalized)
}

fn mail_option_values(
    options: &[String],
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut values = std::collections::HashMap::new();
    let mut index = 0;
    while index < options.len() {
        let value = if options[index] == "-S" {
            index += 1;
            options
                .get(index)
                .map(String::as_str)
                .ok_or_else(|| "SMTP option is missing its value".to_owned())?
        } else {
            options[index]
                .strip_prefix("-S")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "SMTP options may only set supported SMTP variables".to_owned())?
        };
        let (name, setting) = value
            .split_once('=')
            .ok_or_else(|| "SMTP variable must use name=value".to_owned())?;
        values.insert(name.to_owned(), setting.to_owned());
        index += 1;
    }
    Ok(values)
}

fn legacy_mail_address(value: &str) -> Option<&str> {
    let value = value.trim();
    if validate_mail_address(value, "SMTP from address").is_ok() {
        return Some(value);
    }
    let (_, rest) = value.rsplit_once('<')?;
    let address = rest.strip_suffix('>')?.trim();
    validate_mail_address(address, "SMTP from address")
        .is_ok()
        .then_some(address)
}

async fn validate_ping_network_targets(ping: &PingConfig) -> Result<(), String> {
    for value in [
        &ping.completion_url,
        &ping.start_url,
        &ping.success_url,
        &ping.failure_url,
    ] {
        if !value.is_empty() {
            validate_public_url(value, "Ping").await?;
        }
    }
    Ok(())
}

async fn validate_mail_network_target(mail: &MailConfig) -> Result<(), String> {
    if let Some(server) = smtp_server_from_options(&mail.smtp_options) {
        validate_public_url(server, "SMTP").await?;
    }
    Ok(())
}

pub fn smtp_server_from_options(options: &[String]) -> Option<&str> {
    let mut index = 0;
    while index < options.len() {
        let value = if options[index] == "-S" {
            index += 1;
            options.get(index).map(String::as_str)
        } else {
            options[index].strip_prefix("-S")
        };
        if let Some((name, server)) = value.and_then(|value| value.split_once('='))
            && matches!(name, "mta" | "smtp")
        {
            return Some(server);
        }
        index += 1;
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub host: String,
    pub port: u16,
    pub addresses: Vec<IpAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub on_start: bool,
    #[serde(default = "default_true")]
    pub on_success: bool,
    #[serde(default = "default_true")]
    pub on_failure: bool,
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

impl Default for PingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            on_start: true,
            on_success: true,
            on_failure: true,
            completion_url: String::new(),
            completion_options: Vec::new(),
            start_url: String::new(),
            start_options: Vec::new(),
            success_url: String::new(),
            success_options: Vec::new(),
            failure_url: String::new(),
            failure_options: Vec::new(),
        }
    }
}

impl PingConfig {
    fn has_endpoint(&self) -> bool {
        [
            &self.completion_url,
            &self.start_url,
            &self.success_url,
            &self.failure_url,
        ]
        .into_iter()
        .any(|value| !value.trim().is_empty())
    }

    fn validate(&self) -> Result<(), String> {
        for value in [
            &self.completion_url,
            &self.start_url,
            &self.success_url,
            &self.failure_url,
        ] {
            if value.is_empty() {
                continue;
            }
            if value.chars().count() > 2048 || value.contains(['\0', '\r', '\n']) {
                return Err("Ping URL is invalid".into());
            }
            let parsed = url::Url::parse(value).map_err(|_| "Ping URL is invalid")?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err("Ping URL must use HTTP or HTTPS".into());
            }
        }
        for options in [
            &self.completion_options,
            &self.start_options,
            &self.success_options,
            &self.failure_options,
        ] {
            validate_ping_options(options)?;
        }
        if self.enabled && !self.has_endpoint() {
            return Err("at least one Ping URL is required when Ping is enabled".into());
        }
        if self.enabled && !(self.on_start || self.on_success || self.on_failure) {
            return Err("at least one Ping event is required when Ping is enabled".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub smtp_options: Vec<String>,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub on_start: bool,
    #[serde(default = "default_true")]
    pub on_success: bool,
    #[serde(default = "default_true")]
    pub on_failure: bool,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_options: Vec::new(),
            to: String::new(),
            on_start: false,
            on_success: true,
            on_failure: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerChanConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub send_key: String,
    #[serde(default = "default_serverchan_channel")]
    pub channel: String,
    #[serde(default = "default_true")]
    pub on_start: bool,
    #[serde(default = "default_true")]
    pub on_success: bool,
    #[serde(default = "default_true")]
    pub on_failure: bool,
}

impl Default for ServerChanConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            send_key: String::new(),
            channel: default_serverchan_channel(),
            on_start: true,
            on_success: true,
            on_failure: true,
        }
    }
}

fn default_serverchan_channel() -> String {
    "wechat".into()
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalNotificationSettings {
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub config: NotificationConfig,
    pub updated_at: DateTime<Utc>,
}

impl Default for GlobalNotificationSettings {
    fn default() -> Self {
        Self {
            confirmed: false,
            config: NotificationConfig::default(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationMigrationWarning {
    pub channel: String,
    pub reason: String,
}

impl NotificationMigrationWarning {
    fn new(channel: &str, reason: String) -> Self {
        Self {
            channel: channel.into(),
            reason,
        }
    }
}

fn validate_options_shape(options: &[String], channel: &str) -> Result<(), String> {
    if options.len() > 64
        || options
            .iter()
            .any(|value| value.len() > 2048 || value.contains(['\0', '\r', '\n']))
    {
        return Err(format!("{channel} options are invalid"));
    }
    Ok(())
}

fn validate_ping_options(options: &[String]) -> Result<(), String> {
    validate_options_shape(options, "Ping")?;
    let mut index = 0;
    while index < options.len() {
        let option = &options[index];
        let takes_value = matches!(
            option.as_str(),
            "-X" | "--request"
                | "-H"
                | "--header"
                | "-d"
                | "--data"
                | "--data-raw"
                | "--data-urlencode"
                | "-A"
                | "--user-agent"
                | "--connect-timeout"
        );
        if !takes_value || index + 1 >= options.len() {
            return Err("Ping options contain an unsupported curl option".into());
        }
        let value = &options[index + 1];
        if value.contains('@') && matches!(option.as_str(), "--data-urlencode")
            || value.starts_with('@')
            || value.contains("=@")
        {
            return Err("Ping options cannot read data from a file".into());
        }
        index += 2;
    }
    Ok(())
}

fn validate_mail_options(options: &[String]) -> Result<(), String> {
    validate_options_shape(options, "SMTP")?;
    let mut variables = std::collections::HashSet::new();
    let mut index = 0;
    while index < options.len() {
        let option = &options[index];
        let value = if option == "-S" {
            if index + 1 >= options.len() || options[index + 1].starts_with('-') {
                return Err("SMTP option is missing its value".into());
            }
            let value = options[index + 1].as_str();
            index += 2;
            value
        } else if let Some(value) = option.strip_prefix("-S")
            && !value.is_empty()
        {
            index += 1;
            value
        } else {
            return Err("SMTP options may only set supported SMTP variables".into());
        };
        validate_mail_variable(value)?;
        let (name, _) = value
            .split_once('=')
            .expect("validated SMTP variables always use name=value");
        let name = if matches!(name, "mta" | "smtp") {
            "smtp-server"
        } else {
            name
        };
        if !variables.insert(name) {
            return Err("SMTP variables may only be set once".into());
        }
    }
    Ok(())
}

fn validate_mail_variable(value: &str) -> Result<(), String> {
    let (name, setting) = value
        .split_once('=')
        .ok_or_else(|| "SMTP variable must use name=value".to_owned())?;
    const ALLOWED: &[&str] = &[
        "mta",
        "smtp",
        "smtp-auth-user",
        "smtp-auth-password",
        "from",
    ];
    if !ALLOWED.contains(&name) {
        return Err("SMTP variable is not allowed".into());
    }
    if matches!(name, "mta" | "smtp") {
        let parsed = url::Url::parse(setting)
            .map_err(|_| "SMTP server must be an smtp or smtps URL".to_owned())?;
        if !matches!(parsed.scheme(), "smtp" | "smtps") || parsed.host_str().is_none() {
            return Err("SMTP server must be an smtp or smtps URL".into());
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err("SMTP server URL cannot contain credentials".into());
        }
    } else if name == "from" {
        validate_mail_address(setting, "SMTP from address")?;
    }
    Ok(())
}

fn validate_mail_recipient(value: &str) -> Result<(), String> {
    validate_mail_address(value, "SMTP recipient")
}

fn validate_mail_address(value: &str, field: &str) -> Result<(), String> {
    let value = value.trim();
    let valid = !value.starts_with('-')
        && !value.contains(['\0', '\r', '\n', ',', ';', ' ', '\t'])
        && value.len() <= 320
        && value.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && !domain.is_empty()
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && domain.contains('.')
                && local
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-'))
                && domain
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        });
    if !valid {
        return Err(format!("{field} must be one email address"));
    }
    Ok(())
}

pub async fn resolve_public_url(value: &str, channel: &str) -> Result<ResolvedTarget, String> {
    let parsed = url::Url::parse(value).map_err(|_| format!("{channel} URL is invalid"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("{channel} URL must have a host"))?;
    if host.eq_ignore_ascii_case("localhost") {
        return Err(format!(
            "{channel} target must use a public network address"
        ));
    }
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| format!("{channel} URL must have a port"))?;
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| format!("{channel} target cannot be resolved"))?
        .collect();
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(format!(
            "{channel} target must use a public network address"
        ));
    }
    let mut addresses: Vec<IpAddr> = addresses.into_iter().map(|address| address.ip()).collect();
    addresses.sort_unstable();
    addresses.dedup();
    Ok(ResolvedTarget {
        host: host.to_owned(),
        port,
        addresses,
    })
}

async fn validate_public_url(value: &str, channel: &str) -> Result<(), String> {
    resolve_public_url(value, channel).await.map(|_| ())
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => {
            if let Some(ip) = ip.to_ipv4_mapped() {
                is_public_ipv4(ip)
            } else {
                is_public_ipv6(ip)
            }
        }
    }
}

fn is_public_ipv6(ip: std::net::Ipv6Addr) -> bool {
    let [a, b, ..] = ip.segments();
    a & 0xe000 == 0x2000
        && !matches!(
            (a, b),
            (0x2001, 0x0000..=0x01ff) | (0x2001, 0x0db8) | (0x2002, _) | (0x3fff, 0x0000..=0x0fff)
        )
}

fn is_public_ipv4(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !matches!(
        (a, b, c),
        (0, _, _)
            | (10, _, _)
            | (100, 64..=127, _)
            | (127, _, _)
            | (169, 254, _)
            | (172, 16..=31, _)
            | (192, 0, 0)
            | (192, 0, 2)
            | (192, 88, 99)
            | (192, 168, _)
            | (198, 18..=19, _)
            | (198, 51, 100)
            | (203, 0, 113)
            | (224..=255, _, _)
    )
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
const fn default_remote_check_concurrency() -> usize {
    DEFAULT_REMOTE_CHECK_CONCURRENCY
}
const fn default_upload_concurrency() -> usize {
    DEFAULT_UPLOAD_CONCURRENCY
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

    fn valid_plan_input() -> PlanInput {
        serde_json::from_value(serde_json::json!({
            "name": "backup",
            "sources": [{ "name": "data", "path": "/data" }],
            "remotes": [{ "name": "remote", "directory": "/backup" }]
        }))
        .unwrap()
    }

    #[test]
    fn remote_check_concurrency_defaults_for_existing_plans() {
        let input = valid_plan_input();

        assert_eq!(
            input.remote_check_concurrency,
            DEFAULT_REMOTE_CHECK_CONCURRENCY
        );
        assert!(input.validate().is_ok());
    }

    #[test]
    fn upload_concurrency_defaults_to_serial_for_existing_plans() {
        let input = valid_plan_input();

        assert_eq!(input.upload_concurrency, DEFAULT_UPLOAD_CONCURRENCY);
        assert!(input.validate().is_ok());
    }

    #[test]
    fn archive_password_hint_defaults_for_existing_plans() {
        let input = valid_plan_input();
        assert!(input.archive.password_hint.is_empty());
    }

    #[test]
    fn archive_password_hint_rejects_multiline_and_oversized_values() {
        for hint in ["line one\nline two".into(), "x".repeat(161)] {
            let mut input = valid_plan_input();
            input.archive.password_hint = hint;
            assert_eq!(
                input.validate().unwrap_err(),
                "archive password hint is invalid"
            );
        }
    }

    #[test]
    fn notification_target_enum_round_trips_with_a_type_discriminator() {
        let target = NotificationTarget {
            id: "mail-home".into(),
            name: "家庭邮箱".into(),
            enabled: true,
            on_start: false,
            on_success: true,
            on_failure: true,
            kind: NotificationTargetKind::Email {
                config: MailTargetConfig {
                    smtp_options: vec!["-S".into(), "mta=smtps://smtp.example".into()],
                    to: "backup@example.com".into(),
                    ..Default::default()
                },
            },
        };
        let value = serde_json::to_value(&target).unwrap();
        assert_eq!(value["type"], "email");
        assert!(value.get("ping").is_none());
        assert_eq!(
            serde_json::from_value::<NotificationTarget>(value).unwrap(),
            target
        );
    }

    #[test]
    fn legacy_email_target_is_normalized_to_standard_smtp_fields() {
        let mut config = NotificationConfig {
            targets: vec![NotificationTarget {
                id: "legacy-mail".into(),
                name: "Email".into(),
                enabled: true,
                on_start: false,
                on_success: true,
                on_failure: true,
                kind: NotificationTargetKind::Email {
                    config: MailTargetConfig {
                        smtp_options: vec![
                            "-S".into(),
                            "mta=smtps://smtp.example.com:465".into(),
                            "-S".into(),
                            "from=sender@example.com".into(),
                            "-S".into(),
                            "smtp-auth-user=sender@example.com".into(),
                            "-S".into(),
                            "smtp-auth-password=secret".into(),
                        ],
                        to: "receiver@example.com".into(),
                        ..Default::default()
                    },
                },
            }],
            ..Default::default()
        };

        config.normalize_email_targets();
        let NotificationTargetKind::Email { config } = &config.targets[0].kind else {
            panic!("expected email target");
        };
        assert_eq!(config.host, "smtp.example.com");
        assert_eq!(config.port, 465);
        assert_eq!(config.security, SmtpSecurity::Tls);
        assert_eq!(config.from, "sender@example.com");
        assert_eq!(config.username, "sender@example.com");
        assert_eq!(config.password, "secret");
        assert!(config.smtp_options.is_empty());
        assert!(
            config
                .as_legacy(&NotificationTarget {
                    id: "mail".into(),
                    name: "Email".into(),
                    enabled: true,
                    on_start: false,
                    on_success: true,
                    on_failure: true,
                    kind: NotificationTargetKind::Email {
                        config: config.clone()
                    },
                })
                .smtp_options
                .iter()
                .any(|value| value == "mta=smtps://smtp.example.com:465")
        );
    }

    #[test]
    fn redacted_secrets_are_never_merged_across_target_variants() {
        let stored = NotificationTarget {
            id: "same-id".into(),
            name: "ntfy".into(),
            enabled: true,
            on_start: false,
            on_success: true,
            on_failure: true,
            kind: NotificationTargetKind::Ntfy {
                config: NtfyTargetConfig {
                    server: "https://ntfy.sh".into(),
                    topic: "backup".into(),
                    token: "secret-token".into(),
                },
            },
        };
        let incoming = NotificationTarget {
            id: "same-id".into(),
            name: "Ping".into(),
            enabled: true,
            on_start: false,
            on_success: true,
            on_failure: true,
            kind: NotificationTargetKind::Ping {
                config: PingTargetConfig {
                    success_url: "https://example.com/hook".into(),
                    ..Default::default()
                },
            },
        };
        let mut config = NotificationConfig {
            targets: vec![incoming],
            ..Default::default()
        };
        let existing = NotificationConfig {
            targets: vec![stored],
            ..Default::default()
        };
        config.merge_redacted_from(&existing);
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(!serialized.contains("secret-token"));
    }

    #[test]
    fn remote_check_concurrency_accepts_documented_range() {
        for concurrency in [1, MAX_REMOTE_CHECK_CONCURRENCY] {
            let mut input = valid_plan_input();
            input.remote_check_concurrency = concurrency;
            assert!(input.validate().is_ok());
        }
    }

    #[test]
    fn remote_check_concurrency_rejects_values_outside_documented_range() {
        for concurrency in [0, MAX_REMOTE_CHECK_CONCURRENCY + 1] {
            let mut input = valid_plan_input();
            input.remote_check_concurrency = concurrency;
            assert_eq!(
                input.validate().unwrap_err(),
                "remote check concurrency must be between 1 and 32"
            );
        }
    }

    #[test]
    fn upload_concurrency_accepts_documented_range() {
        for concurrency in [1, MAX_UPLOAD_CONCURRENCY] {
            let mut input = valid_plan_input();
            input.upload_concurrency = concurrency;
            assert!(input.validate().is_ok());
        }
    }

    #[test]
    fn upload_concurrency_rejects_values_outside_documented_range() {
        for concurrency in [0, MAX_UPLOAD_CONCURRENCY + 1] {
            let mut input = valid_plan_input();
            input.upload_concurrency = concurrency;
            assert_eq!(
                input.validate().unwrap_err(),
                "upload concurrency must be between 1 and 8"
            );
        }
    }

    #[test]
    fn duplicate_rclone_destinations_are_rejected_after_path_normalization() {
        let mut input = valid_plan_input();
        input.remotes.push(RemoteConfig {
            name: "remote".into(),
            directory: "/backup/".into(),
        });

        assert_eq!(
            input.validate().unwrap_err(),
            "rclone destinations must be unique"
        );
    }

    #[test]
    fn ambiguous_rclone_destination_paths_are_rejected() {
        for directory in ["/backup/.", "/backup/../other", "/backup//daily"] {
            let mut input = valid_plan_input();
            input.remotes[0].directory = directory.into();

            assert_eq!(
                input.validate().unwrap_err(),
                "remote directory cannot contain dot segments or repeated separators"
            );
        }
    }

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

    #[test]
    fn notification_validation_rejects_dangerous_options() {
        let mut config = NotificationConfig::default();
        config.ping.enabled = true;
        config.ping.success_url = "file:///etc/passwd".into();
        assert_eq!(
            config.validate().unwrap_err(),
            "Ping URL must use HTTP or HTTPS"
        );

        config.ping.success_url = "https://status.example/token".into();
        config.ping.success_options = vec!["--output=/tmp/leak".into()];
        assert!(config.validate().unwrap_err().contains("unsupported"));

        config.ping.success_options = vec!["--data-binary".into(), "@/etc/passwd".into()];
        assert!(config.validate().unwrap_err().contains("unsupported"));

        config.ping.success_options = vec!["--data-urlencode".into(), "key@/etc/passwd".into()];
        assert!(config.validate().unwrap_err().contains("cannot read"));

        config.ping.success_options.clear();
        config.mail.smtp_options = vec!["-a".into(), "/etc/passwd".into()];
        assert!(config.validate().unwrap_err().contains("may only"));

        config.mail.smtp_options = vec!["-S".into(), "netrc-pipe=touch /tmp/pwn".into()];
        assert!(config.validate().unwrap_err().contains("not allowed"));

        config.mail.smtp_options = vec!["-S".into(), "ssl-verify=ignore".into()];
        assert!(config.validate().unwrap_err().contains("not allowed"));

        config.mail.smtp_options = vec!["-S".into(), "mta=test:///config/key".into()];
        assert!(config.validate().unwrap_err().contains("smtp or smtps"));

        config.mail.smtp_options =
            vec!["-S".into(), "mta=smtps://alice:hunter2@smtp.example".into()];
        assert!(
            config
                .validate()
                .unwrap_err()
                .contains("cannot contain credentials")
        );

        config.mail.smtp_options.clear();
        config.mail.enabled = true;
        config.mail.to = "receiver@example.com".into();
        assert!(
            config
                .validate()
                .unwrap_err()
                .contains("SMTP server is required")
        );
        config.mail.enabled = false;
        config.mail.to = "-X!touch /tmp/pwn".into();
        assert!(config.validate().unwrap_err().contains("one email"));
    }

    #[test]
    fn ordinary_validation_does_not_apply_legacy_mail_conversion() {
        let mut config = NotificationConfig::default();
        config.mail.enabled = true;
        config.mail.to = "receiver@example.com".into();
        config.mail.smtp_options = vec![
            "-S".into(),
            "v15-compat".into(),
            "-S".into(),
            "mta=smtp://smtp.example:587".into(),
            "-S".into(),
            "from=sender@example.com".into(),
        ];

        assert!(config.validate().unwrap_err().contains("name=value"));
    }

    #[test]
    fn smtp_validation_rejects_duplicate_variables_and_server_aliases() {
        let mut config = NotificationConfig::default();
        config.mail.smtp_options = vec![
            "-S".into(),
            "mta=smtp://8.8.8.8:587".into(),
            "-S".into(),
            "mta=smtp://127.0.0.1:587".into(),
        ];
        assert!(config.validate().unwrap_err().contains("only be set once"));

        config.mail.smtp_options[2..]
            .clone_from_slice(&["-S".into(), "smtp=smtp://127.0.0.1:587".into()]);
        assert!(config.validate().unwrap_err().contains("only be set once"));
    }

    #[tokio::test]
    async fn migration_isolates_ambiguous_smtp_without_dropping_other_channels() {
        let mut legacy = NotificationConfig::default();
        legacy.ping.enabled = true;
        legacy.ping.success_url = "https://8.8.8.8/backup-ok".into();
        legacy.mail.enabled = true;
        legacy.mail.to = "receiver@example.com".into();
        legacy.mail.smtp_options = vec![
            "-S".into(),
            "mta=smtp://8.8.8.8:587".into(),
            "-S".into(),
            "mta=smtp://127.0.0.1:587".into(),
            "-S".into(),
            "from=sender@example.com".into(),
        ];
        legacy.serverchan.enabled = true;
        legacy.serverchan.send_key = "SCTexample".into();

        let (isolated, warnings) = legacy.isolate_migration_channels().await;

        assert!(isolated.ping.enabled);
        assert_eq!(isolated.mail, MailConfig::default());
        assert!(isolated.serverchan.enabled);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].channel, "mail");
        assert!(warnings[0].reason.contains("only be set once"));
    }

    #[tokio::test]
    async fn notification_targets_reject_local_networks() {
        let mut config = NotificationConfig::default();
        config.ping.success_url = "http://127.0.0.1:8080/test".into();
        assert!(
            config
                .validate_network_targets()
                .await
                .unwrap_err()
                .contains("public network")
        );
        config.ping.success_url = "http://169.254.169.254/latest/meta-data".into();
        assert!(config.validate_network_targets().await.is_err());
    }

    #[tokio::test]
    async fn migration_isolates_invalid_mail_without_dropping_other_channels() {
        let mut legacy = NotificationConfig::default();
        legacy.ping.enabled = true;
        legacy.ping.success_url = "https://8.8.8.8/backup-ok".into();
        legacy.mail.enabled = true;
        legacy.mail.to = "receiver@example.com".into();
        legacy.mail.smtp_options = vec![
            "-S".into(),
            "mta=smtp://8.8.8.8:587".into(),
            "-S".into(),
            "unsupported-legacy-switch".into(),
        ];
        legacy.serverchan.enabled = true;
        legacy.serverchan.send_key = "SCTexample".into();

        let (isolated, warnings) = legacy.isolate_migration_channels().await;

        assert!(isolated.ping.enabled);
        assert_eq!(isolated.ping.success_url, "https://8.8.8.8/backup-ok");
        assert_eq!(isolated.mail, MailConfig::default());
        assert!(isolated.serverchan.enabled);
        assert_eq!(isolated.serverchan.send_key, "SCTexample");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].channel, "mail");
        assert!(!warnings[0].reason.is_empty());
    }

    #[tokio::test]
    async fn migration_drops_a_ping_channel_with_a_private_target() {
        let mut legacy = NotificationConfig::default();
        legacy.ping.enabled = true;
        legacy.ping.success_url = "http://127.0.0.1/private".into();
        legacy.serverchan.enabled = true;
        legacy.serverchan.send_key = "SCTexample".into();

        let (isolated, warnings) = legacy.isolate_migration_channels().await;

        assert_eq!(isolated.ping, PingConfig::default());
        assert!(isolated.serverchan.enabled);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].channel, "ping");
    }

    #[test]
    fn public_ip_check_rejects_ipv4_mapped_local_addresses() {
        assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("::ffff:169.254.169.254".parse().unwrap()));
        assert!(!is_public_ip("::ffff:10.0.0.1".parse().unwrap()));
        assert!(is_public_ip("::ffff:8.8.8.8".parse().unwrap()));
        assert!(!is_public_ip("100.64.0.1".parse().unwrap()));
        assert!(!is_public_ip("192.0.2.1".parse().unwrap()));
        assert!(!is_public_ip("2001:db8::1".parse().unwrap()));
        assert!(!is_public_ip("2001:2::1".parse().unwrap()));
        assert!(!is_public_ip("2001:10::1".parse().unwrap()));
        assert!(!is_public_ip("2001:20::1".parse().unwrap()));
        assert!(!is_public_ip("2002:0808:0808::1".parse().unwrap()));
        assert!(!is_public_ip("3fff::1".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
    }
}
