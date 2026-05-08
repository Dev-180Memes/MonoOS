use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A capability represents a specific function or feature that an agent can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
  pub id: String,
  pub name: String,
  pub description: String,
  pub inputs: Vec<CapabilityInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInput {
  pub name: String,
  pub kind: String,
  pub required: bool,
}

/// An intent is what the user typed or said.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
  pub id: String,
  pub raw_text: String,
  pub submitted_at: DateTime<Utc>,
}

/// A capability call is one step in a plan.
/// The orchestrator produces a Vec<CapabilityCall> from an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCall {
  pub capability_id: String,
  pub inouts: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
  pub intent_id: String,
  pub steps: Vec<CapabilityCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionResult {
  Success { output: String },
  Failure { reason: String },
}

/// A ledger entry records one executed capability call, along with its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
  pub id: String,
  pub intent_id: String,
  pub capability_id: String,
  pub inputs_snapshot: serde_json::Value,
  pub result: ExecutionResult,
  pub executed_at: DateTime<Utc>,
}