use anyhow::{Context, bail};
use rand::{Rng, distr::Alphanumeric};
use reqwest::Client;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
    time::sleep,
};

#[derive(Clone)]
pub struct RcloneRc {
    client: Client,
    url: String,
    user: String,
    password: String,
    config_path: String,
    version: String,
    ready: Arc<AtomicBool>,
    _child: Arc<Mutex<Child>>,
}

impl RcloneRc {
    pub async fn start(config_path: &str) -> anyhow::Result<Self> {
        let user = random_secret(24);
        let password = random_secret(48);
        let child = Command::new("rclone")
            .args([
                "rcd",
                "--config",
                config_path,
                "--rc-addr",
                "127.0.0.1:5572",
                "--rc-user",
                &user,
                "--rc-pass",
                &password,
                "--rc-job-expire-duration",
                "24h",
                "--rc-job-expire-interval",
                "1m",
            ])
            .kill_on_drop(true)
            .spawn()
            .context("start private rclone RC daemon")?;
        let mut rc = Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build()?,
            url: "http://127.0.0.1:5572".into(),
            user,
            password,
            config_path: config_path.to_owned(),
            version: String::new(),
            ready: Arc::new(AtomicBool::new(false)),
            _child: Arc::new(Mutex::new(child)),
        };
        for _ in 0..50 {
            if let Ok(value) = rc.call("core/version", json!({})).await {
                rc.version = rclone_version(&value).unwrap_or_default().to_owned();
                return Ok(rc);
            }
            sleep(Duration::from_millis(100)).await;
        }
        bail!("private rclone RC daemon did not become ready")
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub async fn refresh_ready(&self) -> bool {
        let ready = self
            .call("config/dump", json!({}))
            .await
            .ok()
            .and_then(|value| value.as_object().map(|remotes| !remotes.is_empty()))
            .unwrap_or(false);
        self.ready.store(ready, Ordering::Relaxed);
        ready
    }

    pub async fn run_command(
        &self,
        command: &str,
        mut args: Vec<String>,
        options: Vec<String>,
    ) -> anyhow::Result<Value> {
        args.extend(options);
        args.extend(["--config".into(), self.config_path.clone()]);
        let started = self
            .call(
                "core/command",
                json!({
                    "command": command, "arg": args, "opt": {}, "returnType": "COMBINED_OUTPUT",
                    "_async": true
                }),
            )
            .await?;
        let jobid = started
            .get("jobid")
            .and_then(Value::as_u64)
            .context("RC response has no jobid")?;
        loop {
            let status = self.call("job/status", json!({ "jobid": jobid })).await?;
            if status
                .get("finished")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                if !status
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    bail!(
                        "{}",
                        status
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("rclone job failed")
                    );
                }
                if let Some(error) = command_output_error(&status) {
                    bail!("{error}");
                }
                return Ok(status);
            }
            sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn run_command_output(
        &self,
        command: &str,
        args: Vec<String>,
        options: Vec<String>,
    ) -> anyhow::Result<String> {
        let status = self.run_command(command, args, options).await?;
        Ok(status
            .pointer("/output/result")
            .or_else(|| status.get("output"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned())
    }

    pub async fn stats(&self) -> anyhow::Result<Value> {
        self.call("core/stats", json!({})).await
    }

    pub fn version(&self) -> Option<&str> {
        (!self.version.is_empty()).then_some(&self.version)
    }

    pub async fn providers(&self) -> anyhow::Result<Value> {
        self.call("config/providers", json!({})).await
    }

    pub async fn remotes(&self) -> anyhow::Result<Value> {
        self.call("config/listremotes", json!({})).await
    }

    pub async fn remote_summaries(&self) -> anyhow::Result<Value> {
        let dump = self.call("config/dump", json!({})).await?;
        let providers = self.call("config/providers", json!({})).await?;
        let summaries: Vec<Value> = dump
            .as_object()
            .map(|remotes| {
                remotes
                    .iter()
                    .map(|(name, config)| {
                        let provider_type = config
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        let secret_fields = provider_secret_fields(&providers, provider_type);
                        let (parameters, configured_secrets) =
                            public_remote_parameters(config, &secret_fields);
                        json!({
                            "name": name,
                            "type": provider_type,
                            "parameters": parameters,
                            "configured_secrets": configured_secrets
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(json!({ "remotes": summaries }))
    }

    pub async fn create_remote(
        &self,
        name: &str,
        provider_type: &str,
        parameters: Value,
    ) -> anyhow::Result<Value> {
        let result = self
            .call(
                "config/create",
                json!({
                    "name": name,
                    "type": provider_type,
                    "parameters": parameters,
                    "opt": {
                        "obscure": true,
                        "nonInteractive": true,
                        "all": false
                    }
                }),
            )
            .await?;
        self.refresh_ready().await;
        Ok(result)
    }

    pub async fn continue_remote(
        &self,
        name: &str,
        provider_type: &str,
        parameters: Value,
        state: &str,
        result: Value,
    ) -> anyhow::Result<Value> {
        let response = self
            .call(
                "config/create",
                json!({
                    "name": name,
                    "type": provider_type,
                    "parameters": parameters,
                    "opt": {
                        "obscure": true,
                        "nonInteractive": true,
                        "continue": true,
                        "state": state,
                        "result": result
                    }
                }),
            )
            .await?;
        self.refresh_ready().await;
        Ok(response)
    }

    pub async fn update_remote(&self, name: &str, parameters: Value) -> anyhow::Result<Value> {
        let result = self
            .call(
                "config/update",
                json!({
                    "name": name,
                    "parameters": parameters,
                    "opt": {
                        "obscure": true,
                        "nonInteractive": true,
                        "all": false
                    }
                }),
            )
            .await?;
        self.refresh_ready().await;
        Ok(result)
    }

    pub async fn delete_remote(&self, name: &str) -> anyhow::Result<Value> {
        let result = self.call("config/delete", json!({ "name": name })).await?;
        self.refresh_ready().await;
        Ok(result)
    }

    async fn call(&self, method: &str, body: Value) -> anyhow::Result<Value> {
        let response = self
            .client
            .post(format!("{}/{}", self.url, method))
            .basic_auth(&self.user, Some(&self.password))
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            bail!("rclone RC {method} returned {status}: {text}");
        }
        Ok(serde_json::from_str(&text)?)
    }
}

fn random_secret(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn rclone_version(value: &Value) -> Option<&str> {
    value.get("version").and_then(Value::as_str)
}

fn command_output_error(status: &Value) -> Option<&str> {
    let output = status.get("output")?;
    let failed = output
        .get("error")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    failed
        .then(|| output.get("result").and_then(Value::as_str))
        .flatten()
        .or(failed.then_some("rclone command failed"))
}

fn provider_secret_fields(providers: &Value, provider_type: &str) -> Vec<String> {
    providers
        .get("providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|provider| {
            ["Name", "name", "Prefix", "prefix"]
                .into_iter()
                .any(|key| provider.get(key).and_then(Value::as_str) == Some(provider_type))
        })
        .and_then(|provider| provider.get("Options").or_else(|| provider.get("options")))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|option| {
            let name = option
                .get("Name")
                .or_else(|| option.get("name"))
                .and_then(Value::as_str)?;
            let is_password = option
                .get("IsPassword")
                .or_else(|| option.get("isPassword"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let is_sensitive = option
                .get("Sensitive")
                .or_else(|| option.get("sensitive"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            (is_password
                || secret_field_name(name)
                || (is_sensitive && !safe_sensitive_identifier(name)))
            .then(|| name.to_owned())
        })
        .collect()
}

fn safe_sensitive_identifier(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "user" | "username" | "email" | "access_key_id" | "account_id" | "client_id"
    )
}

fn secret_field_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized == "pass"
        || normalized == "headers"
        || normalized.ends_with("_pass")
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("token")
        || normalized.contains("api_key")
        || normalized == "key"
        || (normalized.ends_with("_key") && !normalized.ends_with("public_key"))
        || normalized.contains("private_key")
        || normalized.starts_with("private_")
        || normalized.contains("credentials")
        || matches!(
            normalized.as_str(),
            "access_grant"
                | "authorization"
                | "connection_string"
                | "cookies"
                | "key_pem"
                | "master_key"
                | "master_keys"
                | "mnemonic"
                | "sas_url"
        )
}

fn public_remote_parameters(
    config: &Value,
    provider_secret_fields: &[String],
) -> (serde_json::Map<String, Value>, Vec<String>) {
    let mut parameters = serde_json::Map::new();
    let mut configured_secrets = Vec::new();
    let Some(config) = config.as_object() else {
        return (parameters, configured_secrets);
    };

    for (key, value) in config {
        if key == "type" {
            continue;
        }
        if provider_secret_fields.iter().any(|secret| secret == key) || secret_field_name(key) {
            if parameter_has_value(value) {
                configured_secrets.push(key.clone());
            }
        } else if parameter_has_value(value) {
            parameters.insert(key.clone(), value.clone());
        }
    }
    configured_secrets.sort();
    (parameters, configured_secrets)
}

fn parameter_has_value(value: &Value) -> bool {
    !value.is_null() && value.as_str().is_none_or(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        command_output_error, provider_secret_fields, public_remote_parameters, rclone_version,
    };
    use serde_json::json;

    #[test]
    fn reads_version_from_core_version_response() {
        assert_eq!(
            rclone_version(&json!({ "version": "v1.75.0" })),
            Some("v1.75.0")
        );
        assert_eq!(rclone_version(&json!({ "version": 175 })), None);
    }

    #[test]
    fn remote_parameters_keep_editable_values_and_only_report_secret_presence() {
        let providers = json!({
            "providers": [{
                "Name": "webdav",
                "Options": [
                    { "Name": "user", "Sensitive": true, "IsPassword": false },
                    { "Name": "access_key_id", "Sensitive": true, "IsPassword": false },
                    { "Name": "pass", "IsPassword": true },
                    { "Name": "session_id", "Sensitive": true, "IsPassword": false },
                    { "Name": "passphrase", "Sensitive": true, "IsPassword": false }
                ]
            }]
        });
        let secrets = provider_secret_fields(&providers, "webdav");
        let (parameters, configured_secrets) = public_remote_parameters(
            &json!({
                "type": "webdav",
                "url": "https://dav.example.test/",
                "user": "alice@example.test",
                "access_key_id": "visible-identifier",
                "pass": "obscured-password",
                "session_id": "do-not-return",
                "passphrase": "do-not-return-either"
            }),
            &secrets,
        );

        assert_eq!(parameters["url"], "https://dav.example.test/");
        assert_eq!(parameters["user"], "alice@example.test");
        assert_eq!(parameters["access_key_id"], "visible-identifier");
        assert!(!parameters.contains_key("pass"));
        assert!(!parameters.contains_key("session_id"));
        assert!(!parameters.contains_key("passphrase"));
        assert_eq!(configured_secrets, ["pass", "passphrase", "session_id"]);
    }

    #[test]
    fn unknown_secret_shaped_fields_are_never_returned() {
        let (parameters, configured_secrets) = public_remote_parameters(
            &json!({
                "type": "custom",
                "endpoint": "https://storage.example.test",
                "custom_token": "do-not-return",
                "mnemonic": "do-not-return-either",
                "headers": "Authorization,Bearer do-not-return"
            }),
            &[],
        );

        assert_eq!(parameters["endpoint"], "https://storage.example.test");
        assert!(!parameters.contains_key("custom_token"));
        assert_eq!(configured_secrets, ["custom_token", "headers", "mnemonic"]);
    }

    #[test]
    fn empty_options_do_not_add_editing_fields_or_claim_a_secret_is_configured() {
        let (parameters, configured_secrets) = public_remote_parameters(
            &json!({
                "type": "s3",
                "endpoint": "",
                "secret_access_key": "",
                "force_path_style": false
            }),
            &[],
        );

        assert!(!parameters.contains_key("endpoint"));
        assert_eq!(parameters["force_path_style"], false);
        assert!(configured_secrets.is_empty());
    }

    #[test]
    fn command_output_error_detects_failed_inner_rclone_command() {
        assert_eq!(
            command_output_error(&json!({
                "success": true,
                "output": { "error": true, "result": "remote unavailable" }
            })),
            Some("remote unavailable")
        );
        assert_eq!(
            command_output_error(&json!({
                "success": true,
                "output": { "error": false, "result": "ok" }
            })),
            None
        );
    }
}
