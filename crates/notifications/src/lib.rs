use anyhow::{Context, bail};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};
use rclone_backup_core::{
    MailConfig, NotificationConfig, NotificationTarget, NotificationTargetKind,
    NotificationTemplate, NtfyTargetConfig, PingConfig, ResolvedTarget, ServerChanChannel,
    resolve_public_url,
};
use std::{collections::HashMap, process::Stdio};
use tokio::process::Command;

pub use rclone_backup_core::{MailTargetConfig, PingTargetConfig, ServerChanTargetConfig};

#[derive(Debug, Default)]
pub struct DeliveryReport {
    pub messages: Vec<String>,
    pub failed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NotificationVariables<'a> {
    pub content_default: &'a str,
    pub content_en: &'a str,
    pub content_zh: &'a str,
    pub time: &'a str,
    pub backup_size_bytes: Option<u64>,
}

impl DeliveryReport {
    fn success(&mut self, target: &str) {
        self.messages.push(format!("Notification {target} sent."));
    }

    fn warning(&mut self, target: &str, error: impl std::fmt::Display) {
        self.failed = true;
        self.messages
            .push(format!("Notification {target} warning: {error}"));
    }
}

pub async fn deliver(
    plan_name: &str,
    config: &NotificationConfig,
    event: &str,
    content: &str,
) -> DeliveryReport {
    deliver_with_variables(
        plan_name,
        config,
        event,
        NotificationVariables {
            content_default: content,
            content_en: content,
            content_zh: content,
            time: "",
            backup_size_bytes: None,
        },
    )
    .await
}

pub async fn deliver_with_variables(
    plan_name: &str,
    config: &NotificationConfig,
    event: &str,
    variables: NotificationVariables<'_>,
) -> DeliveryReport {
    let mut report = DeliveryReport::default();
    for target in &config.targets {
        if target.enabled && event_enabled(target, event) {
            let template = match config.template_for(&target.template_id) {
                Ok(template) => template,
                Err(error) => {
                    report.warning(&target.name, error);
                    continue;
                }
            };
            let (title, body) =
                render_message_with_variables(plan_name, event, variables, template);
            deliver_target(target, event, &title, &body, &mut report).await;
        }
    }
    report
}

async fn deliver_target(
    target: &NotificationTarget,
    event: &str,
    title: &str,
    content: &str,
    report: &mut DeliveryReport,
) {
    let result = match &target.kind {
        NotificationTargetKind::Ping { config: _ } => {
            send_ping(&target.as_notification_config().ping, event, title, content).await
        }
        NotificationTargetKind::Email { config: _ } => {
            send_mail(&target.as_notification_config().mail, title, content).await
        }
        NotificationTargetKind::ServerChan { config } => {
            send_serverchan(config.channel, &config.send_key, title, content).await
        }
        NotificationTargetKind::Ntfy { config } => send_ntfy(config, title, content).await,
    };
    match result {
        Ok(()) => report.success(&target.name),
        Err(error) => report.warning(&target.name, error),
    }
}

fn event_enabled(target: &NotificationTarget, event: &str) -> bool {
    legacy_event_enabled(target.on_start, target.on_success, target.on_failure, event)
}

fn legacy_event_enabled(start: bool, success: bool, failure: bool, event: &str) -> bool {
    matches!(
        (event, start, success, failure),
        ("start", true, _, _) | ("success", _, true, _) | ("failure", _, _, true)
    )
}

fn subject(plan_name: &str, event: &str) -> String {
    format!(
        "{plan_name} Backup {}",
        match event {
            "start" => "Start",
            "success" => "Success",
            _ => "Failed",
        }
    )
}

#[cfg(test)]
fn render_message(
    plan_name: &str,
    event: &str,
    content: &str,
    template: Option<&NotificationTemplate>,
) -> (String, String) {
    render_message_with_variables(
        plan_name,
        event,
        NotificationVariables {
            content_default: content,
            content_en: content,
            content_zh: content,
            time: "",
            backup_size_bytes: None,
        },
        template,
    )
}

fn render_message_with_variables(
    plan_name: &str,
    event: &str,
    variables: NotificationVariables<'_>,
    template: Option<&NotificationTemplate>,
) -> (String, String) {
    let Some(template) = template else {
        return (
            sanitize_title(&subject(plan_name, event)),
            variables.content_default.to_owned(),
        );
    };
    let language = template.language.as_str();
    let message = template.event(event);
    (
        sanitize_title(&render_value(
            &message.title,
            plan_name,
            localized_event(event, language),
            localized_content(variables, language),
            variables.time,
            &format_backup_size(variables.backup_size_bytes, language),
        )),
        render_value(
            &message.body,
            plan_name,
            localized_event(event, language),
            localized_content(variables, language),
            variables.time,
            &format_backup_size(variables.backup_size_bytes, language),
        ),
    )
}

fn localized_event<'a>(event: &'a str, language: &str) -> &'a str {
    match (language, event) {
        ("zh", "start") => "开始",
        ("zh", "success") => "成功",
        ("zh", "failure") => "失败",
        _ => event,
    }
}

fn localized_content<'a>(variables: NotificationVariables<'a>, language: &str) -> &'a str {
    if language == "zh" {
        variables.content_zh
    } else {
        variables.content_en
    }
}

fn format_backup_size(bytes: Option<u64>, language: &str) -> String {
    let Some(bytes) = bytes else {
        return if language == "zh" {
            "暂不可用"
        } else {
            "Not available"
        }
        .into();
    };
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

fn sanitize_title(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(200)
        .collect()
}

fn render_value(
    template: &str,
    plan_name: &str,
    event: &str,
    content: &str,
    time: &str,
    backup_size: &str,
) -> String {
    let mut output = String::with_capacity(template.len() + content.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        output.push_str(&rest[..open]);
        let placeholder = &rest[open + 2..];
        let Some(close) = placeholder.find("}}") else {
            output.push_str(&rest[open..]);
            return output;
        };
        match &placeholder[..close] {
            "plan_name" => output.push_str(plan_name),
            "event" => output.push_str(event),
            "content" => output.push_str(content),
            "time" => output.push_str(time),
            "backup_size" => output.push_str(backup_size),
            _ => output.push_str(&rest[open..open + close + 4]),
        }
        rest = &placeholder[close + 2..];
    }
    output.push_str(rest);
    output
}

async fn send_ping(
    ping: &PingConfig,
    event: &str,
    subject: &str,
    content: &str,
) -> Result<(), String> {
    let mut endpoints = Vec::new();
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
    let mut sent = false;
    for (url, options) in endpoints.into_iter().filter(|(url, _)| !url.is_empty()) {
        sent = true;
        let url = url
            .replace("%{subject}", &urlencoding(subject))
            .replace("%{content}", &urlencoding(content));
        let mut command = pinned_curl_command(&url, "Ping").await?.args([
            "--noproxy",
            "*",
            "-m",
            "15",
            "-f",
            "--retry",
            "3",
            "--retry-delay",
            "1",
            "-o",
            "/dev/null",
            "-s",
        ]);
        for argument in options {
            command = command.arg(
                argument
                    .replace("%{subject}", subject)
                    .replace("%{content}", content),
            );
        }
        run(command.arg(&url).secret(&url))
            .await
            .map_err(|error| error.to_string())?;
    }
    if sent {
        Ok(())
    } else {
        Err("no URL is configured for this event".into())
    }
}

async fn send_mail(mail: &MailConfig, subject: &str, content: &str) -> Result<(), String> {
    let options = mail_options(&mail.smtp_options)?;
    let server = url::Url::parse(&options.server).map_err(|error| error.to_string())?;
    let host = server
        .host_str()
        .ok_or_else(|| "SMTP server host is required".to_owned())?;
    let port = server
        .port_or_known_default()
        .unwrap_or(if server.scheme() == "smtps" { 465 } else { 587 });
    let resolved = resolve_public_url(&options.server, "SMTP").await?;
    let address = resolved
        .addresses
        .first()
        .ok_or_else(|| "SMTP server did not resolve to a public address".to_owned())?;
    let tls_parameters = TlsParameters::new(host.to_owned()).map_err(|error| error.to_string())?;
    let tls = if server.scheme() == "smtps" {
        Tls::Wrapper(tls_parameters)
    } else {
        Tls::Required(tls_parameters)
    };
    let mut transport =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(address.to_string())
            .port(port)
            .tls(tls)
            .timeout(Some(std::time::Duration::from_secs(30)));
    let message = Message::builder()
        .from(
            options
                .from
                .parse()
                .map_err(|_| "SMTP from address is invalid")?,
        )
        .to(mail.to.parse().map_err(|_| "SMTP recipient is invalid")?)
        .subject(subject)
        .body(content.to_owned())
        .map_err(|error| error.to_string())?;
    if let Some((username, password)) = options.credentials {
        transport = transport.credentials(Credentials::new(username, password));
    }
    transport
        .build()
        .send(message)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn send_serverchan(
    channel: ServerChanChannel,
    send_key: &str,
    subject: &str,
    content: &str,
) -> Result<(), String> {
    let url = if channel == ServerChanChannel::App {
        format!("https://sc3.ft07.com/{send_key}.send")
    } else if let Some(rest) = send_key.strip_prefix("sctp") {
        let number: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        format!("https://{number}.push.ft07.com/send/{send_key}.send")
    } else {
        format!("https://sctapi.ftqq.com/{send_key}.send")
    };
    let command = pinned_curl_command(&url, "ServerChan")
        .await?
        .args([
            "--noproxy",
            "*",
            "-m",
            "15",
            "-f",
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
        .secret(send_key);
    run(command).await.map_err(|error| error.to_string())
}

async fn send_ntfy(config: &NtfyTargetConfig, title: &str, content: &str) -> Result<(), String> {
    let url = format!("{}/{}", config.server.trim_end_matches('/'), config.topic);
    let mut command = pinned_curl_command(&url, "ntfy").await?.args([
        "--noproxy",
        "*",
        "-m",
        "15",
        "-f",
        "--retry",
        "3",
        "-s",
        "-o",
        "/dev/null",
        "-X",
        "POST",
        "-H",
        &format!("Title: {title}"),
        "-d",
        content,
    ]);
    if !config.token.is_empty() {
        let authorization = format!("Authorization: Bearer {}", config.token);
        command = command.arg("-H").arg(&authorization).secret(&config.token);
    }
    run(command.arg(&url).secret(&config.token))
        .await
        .map_err(|error| error.to_string())
}

#[derive(Clone)]
struct CommandSpec {
    args: Vec<String>,
    secrets: Vec<String>,
}

impl CommandSpec {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            secrets: Vec::new(),
        }
    }
    fn arg(mut self, value: impl AsRef<str>) -> Self {
        self.args.push(value.as_ref().into());
        self
    }
    fn args<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.args
            .extend(values.into_iter().map(|value| value.as_ref().into()));
        self
    }
    fn secret(mut self, value: &str) -> Self {
        if !value.is_empty() {
            self.secrets.push(value.into());
        }
        self
    }
}

async fn run(spec: CommandSpec) -> anyhow::Result<()> {
    let mut command = Command::new("curl");
    command
        .args(&spec.args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command.stdin(Stdio::null());
    command.kill_on_drop(true);
    let child = command.spawn().context("start curl")?;
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        let message = redact(&String::from_utf8_lossy(&output.stderr), &spec.secrets);
        bail!("curl exited with {}: {}", output.status, message.trim());
    }
    Ok(())
}

async fn pinned_curl_command(url: &str, channel: &str) -> Result<CommandSpec, String> {
    let target = resolve_public_url(url, channel).await?;
    let scheme = url::Url::parse(url)
        .map_err(|_| format!("{channel} URL is invalid"))?
        .scheme()
        .to_owned();
    if !matches!(scheme.as_str(), "http" | "https" | "smtp" | "smtps") {
        return Err(format!("{channel} URL scheme is not supported"));
    }
    Ok(pinned_command(&target, &scheme))
}

fn pinned_command(target: &ResolvedTarget, scheme: &str) -> CommandSpec {
    let addresses = target
        .addresses
        .iter()
        .map(|address| match address {
            std::net::IpAddr::V4(address) => address.to_string(),
            std::net::IpAddr::V6(address) => format!("[{address}]"),
        })
        .collect::<Vec<_>>()
        .join(",");
    CommandSpec::new().args([
        "--resolve",
        &format!("{}:{}:{addresses}", target.host, target.port),
        "--proto",
        &format!("={scheme}"),
    ])
}

struct MailOptions {
    server: String,
    from: String,
    credentials: Option<(String, String)>,
}

fn mail_options(options: &[String]) -> Result<MailOptions, String> {
    let mut values = HashMap::new();
    let mut index = 0;
    while index < options.len() {
        let value = if options[index] == "-S" {
            index += 1;
            options.get(index).map(String::as_str)
        } else {
            options[index].strip_prefix("-S")
        }
        .ok_or_else(|| "SMTP option is missing its value".to_owned())?;
        let (name, value) = value
            .split_once('=')
            .ok_or_else(|| "SMTP variable must use name=value".to_owned())?;
        values.insert(name, value.to_owned());
        index += 1;
    }
    let server = values
        .remove("mta")
        .or_else(|| values.remove("smtp"))
        .ok_or_else(|| "SMTP server is required".to_owned())?;
    let from = values
        .remove("from")
        .ok_or_else(|| "SMTP from address is required".to_owned())?;
    let credentials = match (
        values.remove("smtp-auth-user"),
        values.remove("smtp-auth-password"),
    ) {
        (Some(user), Some(password)) => Some((user, password)),
        (None, None) => None,
        _ => return Err("SMTP user and password must be configured together".into()),
    };
    Ok(MailOptions {
        server,
        from,
        credentials,
    })
}

fn redact(value: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(value.to_owned(), |output, secret| {
            output.replace(secret, "••••••••")
        })
}

fn urlencoding(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_template_preserves_existing_messages() {
        assert_eq!(
            render_message("Daily", "start", "Start backup at now", None),
            ("Daily Backup Start".into(), "Start backup at now".into())
        );
        assert_eq!(
            render_message("Daily", "success", "Backup completed at now", None),
            (
                "Daily Backup Success".into(),
                "Backup completed at now".into()
            )
        );
        assert_eq!(
            render_message("Daily", "failure", "Backup failed", None),
            ("Daily Backup Failed".into(), "Backup failed".into())
        );
    }

    #[test]
    fn custom_template_renders_unicode_and_repeated_placeholders() {
        let template = rclone_backup_core::NotificationTemplate {
            id: "zh".into(),
            name: "中文".into(),
            language: "zh".into(),
            start: rclone_backup_core::NotificationEventTemplate {
                title: "{{plan_name}} 开始".into(),
                body: "{{content}}".into(),
            },
            success: rclone_backup_core::NotificationEventTemplate {
                title: "{{plan_name}} 成功：{{plan_name}}".into(),
                body: "事件 {{event}}\n{{content}}".into(),
            },
            failure: rclone_backup_core::NotificationEventTemplate {
                title: "{{plan_name}} 失败".into(),
                body: "{{content}}".into(),
            },
        };

        assert_eq!(
            render_message("照片", "success", "完成", Some(&template)),
            ("照片 成功：照片".into(), "事件 成功\n完成".into())
        );
        assert_eq!(
            render_message("{{event}}", "success", "完成", Some(&template)).0,
            "{{event}} 成功：{{event}}"
        );
    }

    #[test]
    fn template_language_localizes_variables_and_renders_time_and_size() {
        let message = rclone_backup_core::NotificationEventTemplate {
            title: "{{event}} · {{time}}".into(),
            body: "{{content}}\n{{backup_size}}".into(),
        };
        let template = rclone_backup_core::NotificationTemplate {
            id: "zh-details".into(),
            name: "中文详情".into(),
            language: "zh".into(),
            start: message.clone(),
            success: message.clone(),
            failure: message,
        };
        let variables = NotificationVariables {
            content_default: "Backup completed at 2026-08-18 12:30:00 +00:00",
            content_en: "Backup completed.",
            content_zh: "备份已完成。",
            time: "2026-08-18 12:30:00 +00:00",
            backup_size_bytes: Some(1_572_864),
        };

        assert_eq!(
            render_message_with_variables("照片", "success", variables, Some(&template)),
            (
                "成功 · 2026-08-18 12:30:00 +00:00".into(),
                "备份已完成。\n1.50 MiB".into(),
            )
        );
    }

    #[test]
    fn rendered_titles_cannot_inject_headers_or_grow_without_bound() {
        let event = rclone_backup_core::NotificationEventTemplate {
            title: "{{content}}".into(),
            body: "{{content}}".into(),
        };
        let template = rclone_backup_core::NotificationTemplate {
            id: "safe-title".into(),
            name: "Safe title".into(),
            language: "en".into(),
            start: event.clone(),
            success: event.clone(),
            failure: event,
        };
        let content = format!("backup failed\r\nX-Injected: yes\0{}", "界".repeat(240));

        let (title, body) = render_message("Daily", "failure", &content, Some(&template));

        assert!(!title.chars().any(char::is_control));
        assert_eq!(title.chars().count(), 200);
        assert_eq!(body, content);

        let (built_in_title, _) = render_message("Daily\r\nX-Injected: yes", "failure", "", None);
        assert!(!built_in_title.chars().any(char::is_control));
    }

    #[test]
    fn targets_can_render_different_templates_from_one_library() {
        let message = |label: &str| rclone_backup_core::NotificationEventTemplate {
            title: format!("{label} {{{{plan_name}}}}"),
            body: format!("{label} {{{{content}}}}"),
        };
        let first = rclone_backup_core::NotificationTemplate {
            id: "first".into(),
            name: "First".into(),
            language: "en".into(),
            start: message("A"),
            success: message("A"),
            failure: message("A"),
        };
        let second = rclone_backup_core::NotificationTemplate {
            id: "second".into(),
            name: "Second".into(),
            language: "en".into(),
            start: message("B"),
            success: message("B"),
            failure: message("B"),
        };

        assert_eq!(
            render_message("Daily", "success", "Done", Some(&first)),
            ("A Daily".into(), "A Done".into())
        );
        assert_eq!(
            render_message("Daily", "success", "Done", Some(&second)),
            ("B Daily".into(), "B Done".into())
        );
    }

    #[test]
    fn curl_is_pinned_to_validated_addresses() {
        let target = ResolvedTarget {
            host: "notify.example".into(),
            port: 443,
            addresses: vec!["8.8.8.8".parse().unwrap()],
        };
        let command = pinned_command(&target, "https");
        assert!(
            command
                .args
                .windows(2)
                .any(|pair| pair == ["--resolve", "notify.example:443:8.8.8.8"])
        );
        assert!(!command.args.iter().any(|argument| argument == "--location"));
    }

    #[test]
    fn target_failures_are_recorded_without_aborting_the_report() {
        let mut report = DeliveryReport::default();
        report.warning("first", "failed");
        report.success("second");
        assert!(report.failed);
        assert_eq!(report.messages.len(), 2);
    }
}
