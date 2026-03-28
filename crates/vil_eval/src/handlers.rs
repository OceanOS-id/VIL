use vil_server::prelude::*;

use crate::{
    AnswerLength, AnswerRelevance, ContextRecall, EvalDataset, EvalReport, EvalRunner, Faithfulness,
};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct EvalRunRequest {
    pub dataset_json: String,
    pub metrics: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct EvalRunResponseBody {
    pub report: EvalReport,
    pub case_count: usize,
}

#[derive(Debug, Serialize)]
pub struct EvalStatsBody {
    pub available_metrics: Vec<String>,
    pub dataset_case_count: usize,
    pub dataset_empty: bool,
    pub version: String,
}

pub async fn run_handler(body: ShmSlice) -> HandlerResult<VilResponse<EvalRunResponseBody>> {
    let req: EvalRunRequest = body.json().expect("invalid JSON");
    let dataset = EvalDataset::from_json(&req.dataset_json)
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    if dataset.is_empty() {
        return Err(VilError::bad_request("dataset must not be empty"));
    }

    let mut runner = EvalRunner::new(dataset);

    let metrics = req.metrics.unwrap_or_else(|| {
        vec![
            "answer_relevance".into(),
            "faithfulness".into(),
            "context_recall".into(),
            "answer_length".into(),
        ]
    });
    for m in &metrics {
        match m.as_str() {
            "answer_relevance" => {
                runner = runner.add_metric(Box::new(AnswerRelevance));
            }
            "faithfulness" => {
                runner = runner.add_metric(Box::new(Faithfulness));
            }
            "context_recall" => {
                runner = runner.add_metric(Box::new(ContextRecall));
            }
            "answer_length" => {
                runner = runner.add_metric(Box::new(AnswerLength::default()));
            }
            other => {
                return Err(VilError::bad_request(format!("unknown metric: {other}")));
            }
        }
    }

    let report = runner.run();
    let case_count = report.case_count();
    Ok(VilResponse::ok(EvalRunResponseBody { report, case_count }))
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<EvalStatsBody>> {
    let dataset = ctx.state::<Arc<EvalDataset>>().expect("EvalDataset");
    Ok(VilResponse::ok(EvalStatsBody {
        available_metrics: vec![
            "answer_relevance".into(),
            "faithfulness".into(),
            "context_recall".into(),
            "answer_length".into(),
        ],
        dataset_case_count: dataset.len(),
        dataset_empty: dataset.is_empty(),
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
