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
        let summaries: Vec<Value> = dump
            .as_object()
            .map(|remotes| {
                remotes
                    .iter()
                    .map(|(name, config)| {
                        json!({
                            "name": name,
                            "type": config.get("type").and_then(Value::as_str).unwrap_or("unknown")
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

#[cfg(test)]
mod tests {
    use super::rclone_version;
    use serde_json::json;

    #[test]
    fn reads_version_from_core_version_response() {
        assert_eq!(
            rclone_version(&json!({ "version": "v1.75.0" })),
            Some("v1.75.0")
        );
        assert_eq!(rclone_version(&json!({ "version": 175 })), None);
    }
}
