use anyhow::Result;
use mono_core::{Capability, Plan, CapabilityCall};
use llame_cpp::{LlamaModel, LlamaParams, SessionParams};

pub struct InferenceEngine {
  model: LlamaModel,
}

impl InferenceEngine {
  pub fn load(model_path: &str) -> Result<Self> {
    let params = LlamaParams::default();
    let model = LlamaModel::load(model_path, params)?;
    Ok(Self { model })
  }

  pub fn plan(&self, intent_text: &str, capabilities: &[Capability]) -> Result<Plan> {
    let capability_list = capabilities
      .iter()
      .map(|c| {
        format!("- id: {}\n  description: {}", c.id, c.description)
      }).collect::<Vec<_>>().join("\n");

    let prompt = format!(
      r#"You are an OS orchestrator. Given a user instruction, output a JSON plan.

        Available capabilities:
        {capability_list}

        User instruction: "{intent_text}"

        Output ONLY a JSON object in this exact format, nothing else:
        {{
          "steps": [
            {{
              "capability_id": "<id from the list above>",
              "inputs": {{
                "<input_name>": "<value>"
              }}
            }}
          ]
        }}

        If no capability fits, output: {{"steps": []}}
        JSON:
      "#,
      capability_list = capability_list,
      intent_text = intent_text
    );

    let mut session = self.model.create_session(SessionParams::default())?;
    session.advance_context(&prompt)?;

    let mut output = String::new();
    let completion = session.start_completing_with(
      llama_cpp::standard_sampler::StandardSampler::default(),
      512,
    )?;

    for token in completion {
      let piece = token?;
      output.push_str(&piece);

      if output.trim_end().ends_with("}") {
        break;
      }
    }

    let raw: serde_json::Value = serde_json::from_str(&output.trim())?;
    let steps_raw = raw.get("steps")
      .and_then(|s| s.as_array())
      .ok_or_else(|| anyhow::anyhow!("No 'steps' array in model output"))?;

    let steps = steps_raw.iter().filter_map(|step| {
      let capability_id = step.get("capability_id")?.as_str()?.to_string();
      let inputs = step.get("inputs").cloned().unwrap_or(serde_json::json!({}));
      Some(CapabilityCall { capability_id, inputs })
    }).collect();

    Ok(Plan {
      intent_id: String::new(),
      steps,
    })
  }
}