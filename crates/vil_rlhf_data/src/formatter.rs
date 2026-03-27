// ── N03: RLHF/DPO Formatters ───────────────────────────────────────
use crate::dataset::PreferenceDataset;

/// Format dataset for DPO (Direct Preference Optimization) training.
/// Each line: `{"prompt": ..., "chosen": ..., "rejected": ...}`
pub fn to_dpo_format(dataset: &PreferenceDataset) -> String {
    dataset
        .pairs
        .iter()
        .filter_map(|pair| {
            serde_json::to_string(&serde_json::json!({
                "prompt": pair.prompt,
                "chosen": pair.chosen,
                "rejected": pair.rejected,
            }))
            .ok()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format dataset for standard RLHF training.
/// Includes metadata and separate chosen/rejected blocks.
pub fn to_rlhf_format(dataset: &PreferenceDataset) -> String {
    dataset
        .pairs
        .iter()
        .filter_map(|pair| {
            serde_json::to_string(&serde_json::json!({
                "prompt": pair.prompt,
                "chosen": [
                    {"role": "user", "content": pair.prompt},
                    {"role": "assistant", "content": pair.chosen},
                ],
                "rejected": [
                    {"role": "user", "content": pair.prompt},
                    {"role": "assistant", "content": pair.rejected},
                ],
                "metadata": pair.metadata,
            }))
            .ok()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dataset() -> PreferenceDataset {
        let mut ds = PreferenceDataset::new();
        ds.add_pair("What is 1+1?", "2", "3");
        ds.add_pair("Capital of France?", "Paris", "London");
        ds
    }

    #[test]
    fn dpo_format_produces_jsonl() {
        let out = to_dpo_format(&sample_dataset());
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"chosen\""));
        assert!(lines[0].contains("\"rejected\""));
    }

    #[test]
    fn rlhf_format_has_roles() {
        let out = to_rlhf_format(&sample_dataset());
        assert!(out.contains("\"role\""));
        assert!(out.contains("\"assistant\""));
        assert!(out.contains("\"user\""));
    }

    #[test]
    fn empty_dataset_formats() {
        let ds = PreferenceDataset::new();
        assert!(to_dpo_format(&ds).is_empty());
        assert!(to_rlhf_format(&ds).is_empty());
    }
}
