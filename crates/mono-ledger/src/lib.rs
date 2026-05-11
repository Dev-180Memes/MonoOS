use mono_core::{ExecutionResult, LedgerEntry};
use anyhow::Result;
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use uuid::Uuid;

pub struct Ledger {
  pool: SqlitePool,
}

impl Ledger {
  pub async fn open(db_path: &str) -> Result<Self> {
    let url = format!("sqlite://{}?mode=rwc", db_path);
    let pool = SqlitePoolOptions::new()
      .max_connections(5)
      .connect(&url)
      .await?;

    sqlx::query(
      "CREATE TABLE IF NOT EXISTS ledger (
        id TEXT PRIMARY KEY,
        intent_id TEXT NOT NULL,
        capability_id TEXT NOT NULL,
        inputs_snapshot TEXT NOT NULL,
        result_kind TEXT NOT NULL,
        result_output TEXT NOT NULL,
        executed_at TEXT NOT NULL
      )"
    )
    .execute(&pool)
    .await?;

    Ok(Self { pool })
  }
  
  pub async fn log(
    &self,
    intent_id: &str,
    capability_id: &str,
    inputs: &serde_json::Value,
    result: &ExecutionResult,
  ) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let inputs_json = serde_json::to_string(inputs)?;
  
    let (kind, output) = match result {
      ExecutionResult::Success { output } => ("success", output.clone()),
      ExecutionResult::Failure { reason } => ("failure", reason.clone()),
    };
  
    sqlx::query(
      "INSERT INTO ledger
      (id, intent_id, capability_id, inputs_snapshot, result_kind, result_output, executed_at)
      VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(intent_id)
    .bind(capability_id)
    .bind(&inputs_json)
    .bind(kind)
    .bind(&output)
    .bind(&now)
    .execute(&self.pool)
    .await?;
  
    Ok(id)
  }

  pub async fn last_entries(&self, n: i64) -> Result<Vec<LedgerEntry>> {
    let rows = sqlx::query_as::<_, LedgerRow>(
      "SELECT * FROM ledger ORDER BY executed_at DESC LIMIT ?"
    )
    .bind(n)
    .fetch_all(&self.pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
  }
}

#[derive(sqlx::FromRow)]
struct LedgerRow {
  id: String,
  intent_id: String,
  capability_id: String,
  inputs_snapshot: String,
  result_kind: String,
  result_output: String,
  executed_at: String,
}

impl From<LedgerRow> for LedgerEntry {
  fn from(r: LedgerRow) -> Self {
    let inputs_snapshot = serde_json::from_str(&r.inputs_snapshot)
      .unwrap_or(serde_json::Value::Null);
    let result = if r.result_kind == "success" {
      ExecutionResult::Success { output: r.result_output }
    } else {
      ExecutionResult::Failure { reason: r.result_output }
    };
    LedgerEntry {
      id: r.id,
      intent_id: r.intent_id,
      capability_id: r.capability_id,
      inputs_snapshot,
      result,
      executed_at: chrono::DateTime::parse_from_rfc3339(&r.executed_at)
        .unwrap()
        .with_timezone(&chrono::Utc),
    }
  }
}
