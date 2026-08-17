use crate::model::*;
use chrono::Utc;
use std::{collections::HashMap, env, fs};
use uuid::Uuid;

pub const LEGACY_KEYS: &[&str] = &[
    "DISPLAY_NAME",
    "CRON",
    "TIMEZONE",
    "RCLONE_REMOTE_NAME",
    "RCLONE_REMOTE_DIR",
    "RCLONE_GLOBAL_FLAG",
    "BACKUP_FOLDER_NAME",
    "BACKUP_FOLDER_PATH",
    "ZIP_ENABLE",
    "ZIP_PASSWORD",
    "ZIP_TYPE",
    "BACKUP_FILE_SUFFIX",
    "BACKUP_FILE_DATE",
    "BACKUP_FILE_DATE_SUFFIX",
    "BACKUP_KEEP_DAYS",
    "BACKUP_KEEP_COUNT",
    "PING_URL",
    "PING_URL_CURL_OPTIONS",
    "PING_URL_WHEN_START",
    "PING_URL_WHEN_START_CURL_OPTIONS",
    "PING_URL_WHEN_SUCCESS",
    "PING_URL_WHEN_SUCCESS_CURL_OPTIONS",
    "PING_URL_WHEN_FAILURE",
    "PING_URL_WHEN_FAILURE_CURL_OPTIONS",
    "MAIL_SMTP_ENABLE",
    "MAIL_SMTP_VARIABLES",
    "MAIL_TO",
    "MAIL_WHEN_SUCCESS",
    "MAIL_WHEN_FAILURE",
    "SERVERCHAN_ENABLE",
    "SERVERCHAN_SENDKEY",
    "SERVERCHAN_WHEN_START",
    "SERVERCHAN_WHEN_SUCCESS",
    "SERVERCHAN_WHEN_FAILURE",
];

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub address: String,
    pub site_name: String,
    pub database_url: String,
    pub work_dir: String,
    pub log_level: String,
    pub public_auth: Option<(String, String)>,
    pub secret_key_file: String,
    pub secret_key: Option<String>,
    pub rclone_config: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let dotenv = parse_dotenv("/.env");
        let user = resolve("RCLONE_BACKUP_USER", &dotenv).unwrap_or_default();
        let password = resolve("RCLONE_BACKUP_PASSWORD", &dotenv).unwrap_or_default();
        let public_auth = match (user.is_empty(), password.is_empty()) {
            (true, true) => None,
            (false, false) => Some((user, password)),
            _ => {
                return Err(
                    "RCLONE_BACKUP_USER and RCLONE_BACKUP_PASSWORD must be set together".into(),
                );
            }
        };
        let site_name = configured_site_name(resolve("RCLONE_BACKUP_SITE_NAME", &dotenv))?;
        Ok(Self {
            address: env::var("RCLONE_BACKUP_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            site_name,
            database_url: env::var("RCLONE_BACKUP_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:///config/rclone-backup.db?mode=rwc".into()),
            work_dir: env::var("RCLONE_BACKUP_WORK_DIR")
                .unwrap_or_else(|_| "/tmp/rclone-backup".into()),
            log_level: env::var("RUST_LOG")
                .unwrap_or_else(|_| "rclone_backup=info,tower_http=info".into()),
            public_auth,
            secret_key_file: env::var("RCLONE_BACKUP_KEY_FILE")
                .unwrap_or_else(|_| "/config/.rclone-backup.key".into()),
            secret_key: resolve("RCLONE_BACKUP_SECRET_KEY", &dotenv),
            rclone_config: env::var("RCLONE_CONFIG")
                .unwrap_or_else(|_| "/config/rclone/rclone.conf".into()),
        })
    }
}

pub fn plans_from_environment() -> Result<Vec<Plan>, String> {
    let dotenv = parse_dotenv("/.env");
    if resolve("DB_TYPE", &dotenv)
        .is_some_and(|value| !value.eq_ignore_ascii_case("none") && !value.is_empty())
    {
        tracing::warn!("legacy DB_TYPE is ignored because v2 backs up directories only");
    }
    if let Some(raw) = resolve("RCLONE_BACKUP_PLANS", &dotenv) {
        let inputs: Vec<PlanInput> =
            serde_json::from_str(&raw).map_err(|e| format!("invalid RCLONE_BACKUP_PLANS: {e}"))?;
        return inputs.into_iter().map(make_plan).collect();
    }
    if has_legacy_configuration(&dotenv) {
        Ok(vec![make_plan(legacy_plan_input(&dotenv))?])
    } else {
        Ok(Vec::new())
    }
}

fn has_legacy_configuration(dotenv: &HashMap<String, String>) -> bool {
    LEGACY_KEYS.iter().any(|key| resolve(key, dotenv).is_some())
        || (1..=100).any(|index| {
            [
                "RCLONE_REMOTE_NAME",
                "RCLONE_REMOTE_DIR",
                "BACKUP_FOLDER_NAME",
                "BACKUP_FOLDER_PATH",
            ]
            .iter()
            .any(|key| resolve(&format!("{key}_{index}"), dotenv).is_some())
        })
}

fn make_plan(input: PlanInput) -> Result<Plan, String> {
    input.validate()?;
    let now = Utc::now();
    Ok(input.into_plan(Uuid::new_v4(), now))
}

fn legacy_plan_input(dotenv: &HashMap<String, String>) -> PlanInput {
    let get = |key: &str| resolve(key, dotenv).unwrap_or_default();
    let display_name = nonempty(get("DISPLAY_NAME"), "RcloneBackup");
    let folder_name = nonempty(get("BACKUP_FOLDER_NAME"), "data");
    let folder_path = nonempty(get("BACKUP_FOLDER_PATH"), "/data");
    let remote_name = nonempty(get("RCLONE_REMOTE_NAME"), "RcloneBackup");
    let remote_dir = nonempty(get("RCLONE_REMOTE_DIR"), "/RcloneBackup/");

    let mut sources = vec![FolderSource {
        name: folder_name,
        path: folder_path.clone(),
    }];
    for index in 1..=100 {
        let name = get(&format!("BACKUP_FOLDER_NAME_{index}"));
        let path = get(&format!("BACKUP_FOLDER_PATH_{index}"));
        if name.is_empty() || path.is_empty() {
            break;
        }
        sources.push(FolderSource { name, path });
    }
    let mut remotes = vec![RemoteConfig {
        name: remote_name,
        directory: remote_dir,
    }];
    for index in 1..=100 {
        let name = get(&format!("RCLONE_REMOTE_NAME_{index}"));
        let directory = get(&format!("RCLONE_REMOTE_DIR_{index}"));
        if name.is_empty() || directory.is_empty() {
            break;
        }
        remotes.push(RemoteConfig { name, directory });
    }

    let suffix = if !get("BACKUP_FILE_SUFFIX").is_empty() {
        get("BACKUP_FILE_SUFFIX").replace('/', "")
    } else {
        format!(
            "{}{}",
            nonempty(get("BACKUP_FILE_DATE"), "%Y%m%d"),
            get("BACKUP_FILE_DATE_SUFFIX")
        )
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '%' | '_' | '-'))
        .collect()
    };
    let zip_enabled = bool_or(&get("ZIP_ENABLE"), true);
    let zip_type = if get("ZIP_TYPE").eq_ignore_ascii_case("7z") {
        "7z"
    } else {
        "zip"
    };
    let mut input = PlanInput {
        name: display_name,
        enabled: true,
        schedule: nonempty(get("CRON"), "5 * * * *"),
        timezone: normalize_legacy_timezone(&nonempty(get("TIMEZONE"), "UTC")),
        sources,
        archive: ArchiveConfig {
            kind: if zip_enabled {
                zip_type.into()
            } else {
                "none".into()
            },
            password: nonempty(get("ZIP_PASSWORD"), "123456"),
            password_hint: String::new(),
            suffix,
        },
        remotes,
        retention: RetentionPolicy {
            keep_days: parse_or(&get("BACKUP_KEEP_DAYS"), 0),
            keep_count: parse_or(&get("BACKUP_KEEP_COUNT"), 0),
        },
        retry: RetryPolicy::default(),
        notifications: NotificationConfig {
            targets: Vec::new(),
            templates: Vec::new(),
            ping: PingConfig {
                enabled: false,
                on_start: true,
                on_success: true,
                on_failure: true,
                completion_url: get("PING_URL"),
                completion_options: split_args(&get("PING_URL_CURL_OPTIONS")),
                start_url: get("PING_URL_WHEN_START"),
                start_options: split_args(&get("PING_URL_WHEN_START_CURL_OPTIONS")),
                success_url: get("PING_URL_WHEN_SUCCESS"),
                success_options: split_args(&get("PING_URL_WHEN_SUCCESS_CURL_OPTIONS")),
                failure_url: get("PING_URL_WHEN_FAILURE"),
                failure_options: split_args(&get("PING_URL_WHEN_FAILURE_CURL_OPTIONS")),
            },
            mail: MailConfig {
                enabled: bool_or(&get("MAIL_SMTP_ENABLE"), false),
                smtp_options: split_args(&get("MAIL_SMTP_VARIABLES")),
                to: get("MAIL_TO"),
                on_start: false,
                on_success: bool_or(&get("MAIL_WHEN_SUCCESS"), true),
                on_failure: bool_or(&get("MAIL_WHEN_FAILURE"), true),
            },
            serverchan: ServerChanConfig {
                enabled: bool_or(&get("SERVERCHAN_ENABLE"), false),
                send_key: get("SERVERCHAN_SENDKEY"),
                channel: "wechat".into(),
                on_start: bool_or(&get("SERVERCHAN_WHEN_START"), true),
                on_success: bool_or(&get("SERVERCHAN_WHEN_SUCCESS"), true),
                on_failure: bool_or(&get("SERVERCHAN_WHEN_FAILURE"), true),
            },
        },
        rclone_flags: split_args(&get("RCLONE_GLOBAL_FLAG")),
        remote_check_concurrency: DEFAULT_REMOTE_CHECK_CONCURRENCY,
        upload_concurrency: DEFAULT_UPLOAD_CONCURRENCY,
    };
    if let Err(reason) = input.notifications.normalize_legacy_mail() {
        tracing::warn!(%reason, "legacy SMTP configuration was disabled during migration");
        input.notifications.mail = MailConfig::default();
    }
    input.notifications.normalize_legacy();
    input.notifications.promote_legacy_targets();
    input
}

fn normalize_legacy_timezone(value: &str) -> String {
    match value.trim() {
        "CST" => "Asia/Shanghai".into(),
        value => value.to_owned(),
    }
}

fn parse_dotenv(path: &str) -> HashMap<String, String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line
                .strip_prefix("export ")
                .unwrap_or(line)
                .split_once('=')?;
            Some((key.trim().to_owned(), unquote(value.trim())))
        })
        .collect()
}

fn resolve(key: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    if let Ok(value) = env::var(key)
        && !value.is_empty()
    {
        return Some(value);
    }
    let file_key = format!("{key}_FILE");
    if let Ok(path) = env::var(&file_key)
        && !path.is_empty()
    {
        return fs::read_to_string(path)
            .ok()
            .map(|v| v.trim_end().to_owned());
    }
    if let Some(path) = dotenv.get(&file_key)
        && !path.is_empty()
    {
        return fs::read_to_string(path)
            .ok()
            .map(|v| v.trim_end().to_owned());
    }
    dotenv.get(key).filter(|v| !v.is_empty()).cloned()
}

fn split_args(value: &str) -> Vec<String> {
    shell_words::split(value).unwrap_or_default()
}
fn nonempty(value: String, default: &str) -> String {
    if value.is_empty() {
        default.into()
    } else {
        value
    }
}
fn bool_or(value: &str, default: bool) -> bool {
    if value.is_empty() {
        default
    } else {
        !matches!(
            value.to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        )
    }
}
fn parse_or<T: std::str::FromStr + Copy>(value: &str, default: T) -> T {
    value.parse().unwrap_or(default)
}
fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
        .to_owned()
}

fn normalize_site_name(value: &str) -> String {
    let value = value.trim();
    let unquoted = [('"', '"'), ('\'', '\''), ('“', '”'), ('‘', '’')]
        .into_iter()
        .find_map(|(opening, closing)| {
            value
                .strip_prefix(opening)
                .and_then(|value| value.strip_suffix(closing))
        })
        .unwrap_or(value);
    unquoted.trim().to_owned()
}

fn configured_site_name(value: Option<String>) -> Result<String, String> {
    let site_name = value
        .as_deref()
        .map(normalize_site_name)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Rclone Backup".into());
    if site_name.chars().count() > 80 {
        return Err("RCLONE_BACKUP_SITE_NAME cannot exceed 80 characters".into());
    }
    Ok(site_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_defaults_validate() {
        let input = legacy_plan_input(&HashMap::new());
        assert!(input.validate().is_ok());
        assert_eq!(input.sources[0].path, "/data");
        assert_eq!(input.remotes[0].name, "RcloneBackup");
    }

    #[test]
    fn legacy_cst_timezone_is_migrated() {
        let dotenv = HashMap::from([("TIMEZONE".into(), "CST".into())]);

        let input = legacy_plan_input(&dotenv);

        assert_eq!(input.timezone, "Asia/Shanghai");
        assert!(input.validate().is_ok());
    }

    #[test]
    fn fresh_install_does_not_create_a_default_plan() {
        assert!(!has_legacy_configuration(&HashMap::new()));
    }

    #[test]
    fn shell_options_are_tokenized_without_execution() {
        assert_eq!(
            split_args("--header 'X-Test: hello world'"),
            ["--header", "X-Test: hello world"]
        );
    }

    #[test]
    fn site_name_removes_only_matching_outer_quotes() {
        assert_eq!(
            normalize_site_name("  \"Vaultwarden+ Backup\"  "),
            "Vaultwarden+ Backup"
        );
        assert_eq!(
            normalize_site_name("'Vaultwarden+ Backup'"),
            "Vaultwarden+ Backup"
        );
        assert_eq!(
            normalize_site_name("“Vaultwarden+ 备份”"),
            "Vaultwarden+ 备份"
        );
        assert_eq!(
            normalize_site_name("‘Vaultwarden+ 备份’"),
            "Vaultwarden+ 备份"
        );
        assert_eq!(
            normalize_site_name("Vaultwarden+ \"Backup\""),
            "Vaultwarden+ \"Backup\""
        );
        assert_eq!(
            normalize_site_name("“Vaultwarden+ Backup"),
            "“Vaultwarden+ Backup"
        );
        assert_eq!(
            normalize_site_name("\"Vaultwarden+ Backup’"),
            "\"Vaultwarden+ Backup’"
        );
        assert_eq!(
            normalize_site_name("“Vaultwarden \"Backup\"”"),
            "Vaultwarden \"Backup\""
        );
    }

    #[test]
    fn site_name_falls_back_after_unquoting_and_checks_normalized_length() {
        assert_eq!(
            configured_site_name(Some("  “  ”  ".into())).unwrap(),
            "Rclone Backup"
        );

        let maximum = "备".repeat(80);
        assert_eq!(
            configured_site_name(Some(format!("“{maximum}”"))).unwrap(),
            maximum
        );
        assert!(configured_site_name(Some(format!("\"{}\"", "备".repeat(81)))).is_err());
    }

    #[test]
    fn legacy_s_nail_mail_options_are_converted_to_standard_smtp_fields() {
        let dotenv = HashMap::from([
            ("MAIL_SMTP_ENABLE".into(), "true".into()),
            ("MAIL_TO".into(), "receiver@example.com".into()),
            (
                "MAIL_SMTP_VARIABLES".into(),
                "-S v15-compat -S mta=smtp://smtp.example:587 -S smtp-use-starttls -S smtp-auth=login -S user=alice@example.com -S password=test-secret -S 'from=Backup <alice@example.com>'".into(),
            ),
        ]);
        let input = legacy_plan_input(&dotenv);

        let NotificationTargetKind::Email { config } = &input.notifications.targets[0].kind else {
            panic!("legacy mail must become an Email target");
        };
        assert_eq!(config.host, "smtp.example");
        assert_eq!(config.port, 587);
        assert_eq!(config.security, rclone_backup_core::SmtpSecurity::Starttls);
        assert_eq!(config.from, "alice@example.com");
        assert_eq!(config.username, "alice@example.com");
        assert_eq!(config.password, "test-secret");
        assert_eq!(config.to, "receiver@example.com");
        assert!(config.smtp_options.is_empty());
        assert!(input.validate().is_ok());
    }

    #[test]
    fn first_environment_import_creates_one_record_for_each_notification_type() {
        let dotenv = HashMap::from([
            ("PING_URL_WHEN_SUCCESS".into(), "https://8.8.8.8/ok".into()),
            ("MAIL_SMTP_ENABLE".into(), "true".into()),
            ("MAIL_TO".into(), "receiver@example.com".into()),
            (
                "MAIL_SMTP_VARIABLES".into(),
                "-S mta=smtps://smtp.example -S from=sender@example.com".into(),
            ),
            ("SERVERCHAN_ENABLE".into(), "true".into()),
            ("SERVERCHAN_SENDKEY".into(), "SCTexample".into()),
        ]);
        let input = legacy_plan_input(&dotenv);
        assert_eq!(input.notifications.targets.len(), 3);
        assert!(matches!(
            input.notifications.targets[0].kind,
            NotificationTargetKind::Ping { .. }
        ));
        assert!(matches!(
            input.notifications.targets[1].kind,
            NotificationTargetKind::Email { .. }
        ));
        assert!(matches!(
            input.notifications.targets[2].kind,
            NotificationTargetKind::ServerChan { .. }
        ));
        assert!(input.notifications.ping == PingConfig::default());
        assert!(input.notifications.mail == MailConfig::default());
        assert!(input.notifications.serverchan == ServerChanConfig::default());
    }
}
