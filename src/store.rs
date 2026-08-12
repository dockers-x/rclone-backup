use crate::model::{Plan, RunRecord};
use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::{fs, path::Path};
use uuid::Uuid;

const ENCRYPTED_PREFIX: &str = "enc:v1:";

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
    cipher: Aes256Gcm,
}

impl Store {
    pub async fn connect(
        url: &str,
        key_file: &str,
        key_override: Option<&str>,
    ) -> anyhow::Result<Self> {
        let key = load_or_create_key(key_file, key_override)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .context("connect SQLite")?;
        let store = Self {
            pool,
            cipher: Aes256Gcm::new_from_slice(&key).expect("AES-256 key length"),
        };
        store.migrate().await?;
        store.encrypt_existing_plans().await?;
        Ok(store)
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS plans (id TEXT PRIMARY KEY, name TEXT NOT NULL, enabled INTEGER NOT NULL, document TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)").execute(&self.pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, plan_id TEXT NOT NULL, plan_name TEXT NOT NULL, trigger TEXT NOT NULL, status TEXT NOT NULL, attempt INTEGER NOT NULL, started_at TEXT NOT NULL, finished_at TEXT, log TEXT NOT NULL DEFAULT '')").execute(&self.pool).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_runs_plan_started ON runs(plan_id, started_at DESC)",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn seed_once(&self, plans: &[Plan]) -> anyhow::Result<bool> {
        let mut tx = self.pool.begin().await?;
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT value FROM metadata WHERE key = 'environment_imported'")
                .fetch_optional(&mut *tx)
                .await?;
        if existing.is_some() {
            return Ok(false);
        }
        for plan in plans {
            let document = self.encode_plan(plan)?;
            sqlx::query("INSERT INTO plans(id,name,enabled,document,created_at,updated_at) VALUES(?,?,?,?,?,?)")
                .bind(plan.id.to_string()).bind(&plan.name).bind(plan.enabled).bind(document).bind(plan.created_at.to_rfc3339()).bind(plan.updated_at.to_rfc3339()).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO metadata(key,value) VALUES('environment_imported',?)")
            .bind(Utc::now().to_rfc3339())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn list_plans(&self) -> anyhow::Result<Vec<Plan>> {
        let docs: Vec<(String,)> =
            sqlx::query_as("SELECT document FROM plans ORDER BY created_at ASC")
                .fetch_all(&self.pool)
                .await?;
        docs.into_iter()
            .map(|(doc,)| self.decode_plan(&doc))
            .collect()
    }

    pub async fn get_plan(&self, id: Uuid) -> anyhow::Result<Option<Plan>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT document FROM plans WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|(doc,)| self.decode_plan(&doc)).transpose()
    }

    pub async fn save_plan(&self, plan: &Plan) -> anyhow::Result<()> {
        let document = self.encode_plan(plan)?;
        sqlx::query("INSERT INTO plans(id,name,enabled,document,created_at,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,enabled=excluded.enabled,document=excluded.document,updated_at=excluded.updated_at")
            .bind(plan.id.to_string()).bind(&plan.name).bind(plan.enabled).bind(document).bind(plan.created_at.to_rfc3339()).bind(plan.updated_at.to_rfc3339()).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_plan(&self, id: Uuid) -> anyhow::Result<bool> {
        Ok(sqlx::query("DELETE FROM plans WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    pub async fn start_run(&self, plan: &Plan, trigger: &str) -> anyhow::Result<RunRecord> {
        let run = RunRecord {
            id: Uuid::new_v4().to_string(),
            plan_id: plan.id.to_string(),
            plan_name: plan.name.clone(),
            trigger: trigger.into(),
            status: "running".into(),
            attempt: 1,
            started_at: Utc::now(),
            finished_at: None,
            log: String::new(),
        };
        sqlx::query("INSERT INTO runs(id,plan_id,plan_name,trigger,status,attempt,started_at,log) VALUES(?,?,?,?,?,?,?,?)")
            .bind(&run.id).bind(&run.plan_id).bind(&run.plan_name).bind(&run.trigger).bind(&run.status).bind(run.attempt).bind(run.started_at.to_rfc3339()).bind("").execute(&self.pool).await?;
        Ok(run)
    }

    pub async fn update_run(
        &self,
        id: &str,
        status: &str,
        attempt: u32,
        log: &str,
        finished: bool,
    ) -> anyhow::Result<()> {
        let finished_at = finished.then(Utc::now);
        sqlx::query("UPDATE runs SET status=?,attempt=?,log=?,finished_at=? WHERE id=?")
            .bind(status)
            .bind(attempt as i64)
            .bind(log)
            .bind(finished_at.map(|value| value.to_rfc3339()))
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_runs(
        &self,
        plan_id: Option<Uuid>,
        limit: u32,
    ) -> anyhow::Result<Vec<RunRecord>> {
        let limit = limit.clamp(1, 200);
        let rows: Vec<RunRow> = if let Some(id) = plan_id {
            sqlx::query_as("SELECT id,plan_id,plan_name,trigger,status,attempt,started_at,finished_at,log FROM runs WHERE plan_id=? ORDER BY started_at DESC LIMIT ?")
                .bind(id.to_string())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT id,plan_id,plan_name,trigger,status,attempt,started_at,finished_at,log FROM runs ORDER BY started_at DESC LIMIT ?")
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        };
        rows.into_iter().map(run_from_row).collect()
    }

    fn encode_plan(&self, plan: &Plan) -> anyhow::Result<String> {
        let plaintext = serde_json::to_vec(plan)?;
        let mut nonce = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let encrypted = self
            .cipher
            .encrypt((&nonce).into(), plaintext.as_ref())
            .map_err(|_| anyhow::anyhow!("encrypt backup plan"))?;
        let mut payload = Vec::with_capacity(nonce.len() + encrypted.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&encrypted);
        Ok(format!(
            "{ENCRYPTED_PREFIX}{}",
            STANDARD_NO_PAD.encode(payload)
        ))
    }

    fn decode_plan(&self, document: &str) -> anyhow::Result<Plan> {
        let Some(encoded) = document.strip_prefix(ENCRYPTED_PREFIX) else {
            return serde_json::from_str(document).context("decode legacy plaintext plan");
        };
        let payload = STANDARD_NO_PAD
            .decode(encoded)
            .context("decode encrypted plan")?;
        if payload.len() < 13 {
            bail!("encrypted plan is truncated");
        }
        let plaintext = self
            .cipher
            .decrypt((&payload[..12]).into(), &payload[12..])
            .map_err(|_| anyhow::anyhow!("decrypt backup plan: secret key is incorrect"))?;
        serde_json::from_slice(&plaintext).context("decode decrypted plan")
    }

    async fn encrypt_existing_plans(&self) -> anyhow::Result<()> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, document FROM plans WHERE document NOT LIKE 'enc:v1:%'")
                .fetch_all(&self.pool)
                .await?;
        for (id, document) in rows {
            let plan: Plan = serde_json::from_str(&document).context("decode legacy plan")?;
            sqlx::query("UPDATE plans SET document=? WHERE id=?")
                .bind(self.encode_plan(&plan)?)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

type RunRow = (
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    Option<String>,
    String,
);

fn run_from_row(row: RunRow) -> anyhow::Result<RunRecord> {
    Ok(RunRecord {
        id: row.0,
        plan_id: row.1,
        plan_name: row.2,
        trigger: row.3,
        status: row.4,
        attempt: row.5,
        started_at: chrono::DateTime::parse_from_rfc3339(&row.6)?.with_timezone(&Utc),
        finished_at: row
            .7
            .map(|value| {
                chrono::DateTime::parse_from_rfc3339(&value).map(|date| date.with_timezone(&Utc))
            })
            .transpose()?,
        log: row.8,
    })
}

fn load_or_create_key(path: &str, key_override: Option<&str>) -> anyhow::Result<[u8; 32]> {
    if let Some(value) = key_override {
        return decode_key(value).context("RCLONE_BACKUP_SECRET_KEY");
    }
    let path = Path::new(path);
    if path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let metadata = fs::symlink_metadata(path)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                bail!("plan encryption key must be a regular file");
            }
            if metadata.uid() != rustix::process::geteuid().as_raw() {
                bail!("plan encryption key must be owned by the current user");
            }
            if metadata.permissions().mode() & 0o077 != 0 {
                bail!("plan encryption key permissions must be 0600");
            }
        }
        return decode_key(fs::read_to_string(path)?.trim()).context("read plan encryption key");
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut key = [0_u8; 32];
    OsRng.fill_bytes(&mut key);
    let encoded = STANDARD_NO_PAD.encode(key);
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(encoded.as_bytes())?;
    }
    #[cfg(not(unix))]
    fs::write(path, encoded)?;
    Ok(key)
}

fn decode_key(value: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = STANDARD_NO_PAD
        .decode(value)
        .context("key must be unpadded base64")?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("key must decode to exactly 32 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::plans_from_environment;

    #[tokio::test]
    async fn seed_is_idempotent_and_plans_persist() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::connect(
            "sqlite::memory:",
            directory.path().join("key").to_str().unwrap(),
            None,
        )
        .await
        .unwrap();
        let plans = plans_from_environment().unwrap();
        assert!(store.seed_once(&plans).await.unwrap());
        assert!(!store.seed_once(&plans).await.unwrap());
        assert_eq!(store.list_plans().await.unwrap().len(), plans.len());
    }

    #[tokio::test]
    async fn plan_documents_are_encrypted_at_rest() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::connect(
            "sqlite::memory:",
            directory.path().join("key").to_str().unwrap(),
            None,
        )
        .await
        .unwrap();
        let mut plans = plans_from_environment().unwrap();
        if plans.is_empty() {
            let now = Utc::now();
            let input = crate::model::PlanInput {
                name: "secret-test".into(),
                enabled: true,
                schedule: "5 * * * *".into(),
                timezone: "UTC".into(),
                sources: vec![crate::model::FolderSource {
                    name: "data".into(),
                    path: "/data".into(),
                }],
                archive: crate::model::ArchiveConfig {
                    kind: "7z".into(),
                    password: "never-plaintext".into(),
                    suffix: "%Y%m%d".into(),
                },
                remotes: vec![crate::model::RemoteConfig {
                    name: "remote".into(),
                    directory: "/backup".into(),
                }],
                retention: Default::default(),
                retry: Default::default(),
                notifications: Default::default(),
                rclone_flags: vec![],
            };
            plans.push(input.into_plan(Uuid::new_v4(), now));
        }
        plans[0].archive.password = "never-plaintext".into();
        store.save_plan(&plans[0]).await.unwrap();
        let (document,): (String,) = sqlx::query_as("SELECT document FROM plans LIMIT 1")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert!(document.starts_with(ENCRYPTED_PREFIX));
        assert!(!document.contains("never-plaintext"));
        assert_eq!(
            store
                .get_plan(plans[0].id)
                .await
                .unwrap()
                .unwrap()
                .archive
                .password,
            "never-plaintext"
        );
    }
}
