use anyhow::{Context, bail};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};
use rclone_backup_core::{
    MailConfig, NotificationConfig, NotificationTarget, NotificationTargetKind, NtfyTargetConfig,
    PingConfig, ResolvedTarget, ServerChanChannel, resolve_public_url,
};
use std::{collections::HashMap, process::Stdio};
use tokio::process::Command;

pub use rclone_backup_core::{MailTargetConfig, PingTargetConfig, ServerChanTargetConfig};

#[derive(Debug, Default)]
pub struct DeliveryReport {
    pub messages: Vec<String>,
    pub failed: bool,
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
    let mut report = DeliveryReport::default();
    for target in &config.targets {
        if target.enabled && event_enabled(target, event) {
            deliver_target(target, plan_name, event, content, &mut report).await;
        }
    }
    report
}

async fn deliver_target(
    target: &NotificationTarget,
    plan_name: &str,
    event: &str,
    content: &str,
    report: &mut DeliveryReport,
) {
    let result = match &target.kind {
        NotificationTargetKind::Ping { config: _ } => {
            send_ping(
                &target.as_notification_config().ping,
                plan_name,
                event,
                content,
            )
            .await
        }
        NotificationTargetKind::Email { config: _ } => {
            send_mail(
                &target.as_notification_config().mail,
                plan_name,
                event,
                content,
            )
            .await
        }
        NotificationTargetKind::ServerChan { config } => {
            send_serverchan(config.channel, &config.send_key, plan_name, event, content).await
        }
        NotificationTargetKind::Ntfy { config } => {
            send_ntfy(config, plan_name, event, content).await
        }
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

async fn send_ping(
    ping: &PingConfig,
    plan_name: &str,
    event: &str,
    content: &str,
) -> Result<(), String> {
    let subject = subject(plan_name, event);
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
            .replace("%{subject}", &urlencoding(&subject))
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
                    .replace("%{subject}", &subject)
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

async fn send_mail(
    mail: &MailConfig,
    plan_name: &str,
    event: &str,
    content: &str,
) -> Result<(), String> {
    let options = mail_options(&mail.smtp_options)?;
    let subject = subject(plan_name, event);
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
    plan_name: &str,
    event: &str,
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
    let subject = subject(plan_name, event);
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

async fn send_ntfy(
    config: &NtfyTargetConfig,
    plan_name: &str,
    event: &str,
    content: &str,
) -> Result<(), String> {
    let url = format!("{}/{}", config.server.trim_end_matches('/'), config.topic);
    let title = subject(plan_name, event);
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
