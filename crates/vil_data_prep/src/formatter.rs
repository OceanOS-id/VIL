// ── N01: Output Formatters ───────────────────────────────────────────
use serde::{Deserialize, Serialize};

/// Supported fine-tuning output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Jsonl,
    Alpaca,
    ShareGPT,
    ChatML,
}

/// A single training record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecord {
    pub instruction: String,
    pub input: String,
    pub output: String,
}

impl TrainingRecord {
    pub fn new(instruction: impl Into<String>, input: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            instruction: instruction.into(),
            input: input.into(),
            output: output.into(),
        }
    }
}

/// Format records as JSONL (one JSON object per line).
pub fn to_jsonl(records: &[TrainingRecord]) -> String {
    records
        .iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format records as Alpaca-style JSON array.
pub fn to_alpaca(records: &[TrainingRecord]) -> String {
    let alpaca: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            serde_json::json!({
                "instruction": r.instruction,
                "input": r.input,
                "output": r.output,
            })
        })
        .collect();
    serde_json::to_string_pretty(&alpaca).unwrap_or_default()
}

/// Format records as ShareGPT conversation format.
pub fn to_sharegpt(records: &[TrainingRecord]) -> String {
    let convos: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            let mut messages = vec![
                serde_json::json!({"from": "human", "value": format!("{}\n{}", r.instruction, r.input).trim().to_string()}),
                serde_json::json!({"from": "gpt", "value": r.output}),
            ];
            if messages[0]["value"].as_str().map_or(false, |s| s.is_empty()) {
                messages[0] = serde_json::json!({"from": "human", "value": &r.instruction});
            }
            serde_json::json!({"conversations": messages})
        })
        .collect();
    serde_json::to_string_pretty(&convos).unwrap_or_default()
}

/// Format records as ChatML.
pub fn to_chatml(records: &[TrainingRecord]) -> String {
    records
        .iter()
        .map(|r| {
            let user_msg = if r.input.is_empty() {
                r.instruction.clone()
            } else {
                format!("{}\n{}", r.instruction, r.input)
            };
            format!(
                "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>",
                user_msg, r.output
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Dispatch to the appropriate formatter.
pub fn format_records(records: &[TrainingRecord], fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Jsonl => to_jsonl(records),
        OutputFormat::Alpaca => to_alpaca(records),
        OutputFormat::ShareGPT => to_sharegpt(records),
        OutputFormat::ChatML => to_chatml(records),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_records() -> Vec<TrainingRecord> {
        vec![
            TrainingRecord::new("Summarize this", "The cat sat on the mat.", "A cat sat on a mat."),
            TrainingRecord::new("Translate to French", "Hello", "Bonjour"),
        ]
    }

    #[test]
    fn jsonl_format() {
        let out = to_jsonl(&sample_records());
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Summarize"));
    }

    #[test]
    fn alpaca_format() {
        let out = to_alpaca(&sample_records());
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["instruction"], "Summarize this");
    }

    #[test]
    fn chatml_format() {
        let out = to_chatml(&sample_records());
        assert!(out.contains("<|im_start|>user"));
        assert!(out.contains("<|im_start|>assistant"));
    }

    #[test]
    fn format_dispatch() {
        let records = sample_records();
        let jsonl = format_records(&records, OutputFormat::Jsonl);
        let alpaca = format_records(&records, OutputFormat::Alpaca);
        assert_ne!(jsonl, alpaca);
    }
}
