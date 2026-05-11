use mono_core::{Capability, CapabilityCall, ExecutionResult};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;

pub. struct CapabilityRegistry {
  capabilities: HashMap<String, Capability>,
}

impl CapabilityRegistry {
  pub fn load_from_dir(dir: &Path) -> Result<Self> {
    let mut capabilities = HashMap::new();

    for entry in std::fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.extension().and_then(|e| e.to_str()) == Some("toml") {
        let content = std::fs::read_to_string(&path)?;
        let cap: Capability = toml::from_str(&content)?;
        capabilities.insert(cap.id.clone(), cap);
      }
    }

    Ok(Self { capabilities })
  }

  pub fn all(&self) -> Vec<&Capability> {
    self.capabilities.values().collect()
  }

  pub fn get(&self, id: &str) -> Option<&Capability> {
    self.capabilities.get(id)
  }

  pub async fn execute(&self, call: &CapabilityCall) -> ExecutionResult {
    match call.capability_id.as_str() {
      "com.mono.tools.summarise-file" => {
        let path = match call.inputs.get("path").and_then(|v| v.as_str()) {
          Some(p) => p.to_string(),
          None => return ExecutionResult::Failure {
            reason: "Missing required input: path".into()
          },
        };
        execute_summarise_file(&path).await
      }
      "com.mono.tools.search-documents" => {
        let query = match call.inputs.get("query").and_then(|v| v.as_str()) {
          Some(q) => q.to_string(),
          None => return ExecutionResult::Failure {
            reason: "Missing required input: query".into()
          },
        };
        let dir = call.inputs.get("directory")
          .and_then(|v| v.as_str())
          .unwrap_or(".")
          .to_string();
        execute_search_documents(&query, &dir).await
      }
      "com.mono.tools.draft-text" => {
        let instructions = match call.inputs.get("instructions").and_then(|v| v.as_str()) {
          Some(i) => i.to_string(),
          None => return ExecutionResult::Failure {
            reason: "Missing required input: instructions".into()
          },
        };
        ExecutionResult::Success {
          output: format!("[Draft requested: {}]\nImplement with model call in mono-inference.", instructions)
        }
      }
      "com.mono.tools.batch-rename" => {
        ExecutionResult::Success {
          output: "Batch rename: stub - implement filesystem logic here.".into()
        }
      }
      "com.mono.tools.explain-last-action" => {
        ExecutionResult::Success {
          output: "explain-last-action: will query ledger - implement after ledger is wired.".into()
        }
      }
      unknown => ExecutionResult::Failure {
        reason: format!("Unknown capability: {}", unknown)
      },
    }
  }
}

async fn execute_summarise_file(path: &str) -> ExecutionResult {
  match std::fs::read_to_string(path) {
    Ok(content) => {
      let preview = &content[..content.len().min(500)];
      ExecutionResult::Success {
        output: format!("File content preview (first 500 chars):\n{}", preview)
      }
    }
    Err(e) => ExecutionResult::Failure {
      reason: format!("Could not read file: {}", e)
    },
  }
}

async fn execute_search_documents(query: &str, dir: &str) -> ExecutionResult {
  let query_lower = query.to_lowercase();
  let mut matches = Vec::new();

  let read_dir = match std::fs::read_dir(dir) {
    Ok(rd) => rd,
    Err(e) => return ExecutionResult::Failure {
      reason: format!("Could not read directory: {}", e)
    },
  };

  for entry in read_dir.flatten() {
    let path = entry.path();
    if path.is_file() {
      if let Ok(content) = std::fs::read_to_string(&path) {
        if content.to_lowercase().contains(&query_lower) {
          matches.push(path.to_string_lossy().to_string());
        }
      }
    }
  }

  if matches.is_empty() {
    ExecutionResult::Success {
      output: format!("No files found matching '{}'.", query)
    }
  } else {
    ExecutionResult::Success {
      output: format!("Found {} file(s) matching '{}':\n{}",
        matches.len(),
        query,
        matches.join("\n")
      )
    }
  }
}