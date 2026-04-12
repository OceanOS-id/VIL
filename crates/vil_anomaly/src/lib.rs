use serde_json::{json, Value};

pub fn is_anomaly(args: &[Value]) -> Result<Value, String> {
    let value = args
        .get(0)
        .and_then(|v| v.as_f64())
        .ok_or("is_anomaly: value required")?;
    let history = args
        .get(1)
        .and_then(|v| v.as_array())
        .ok_or("is_anomaly: history array required")?;
    let method = args.get(2).and_then(|v| v.as_str()).unwrap_or("zscore");
    let threshold = args.get(3).and_then(|v| v.as_f64()).unwrap_or(3.0);

    let vals: Vec<f64> = history.iter().filter_map(|v| v.as_f64()).collect();
    if vals.len() < 3 {
        return Ok(json!({"anomaly": false, "reason": "insufficient history"}));
    }

    let mean: f64 = vals.iter().sum::<f64>() / vals.len() as f64;
    let std: f64 = (vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / vals.len() as f64)
        .sqrt();

    match method {
        "zscore" => {
            let z = if std > 0.0 {
                (value - mean).abs() / std
            } else {
                0.0
            };
            Ok(json!({
                "anomaly": z > threshold,
                "score": z,
                "method": "zscore",
                "threshold": threshold,
                "mean": mean,
                "std": std
            }))
        }
        "iqr" => {
            let mut sorted = vals.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q1 = sorted[sorted.len() / 4];
            let q3 = sorted[3 * sorted.len() / 4];
            let iqr = q3 - q1;
            let lower = q1 - threshold * iqr;
            let upper = q3 + threshold * iqr;
            let anomaly = value < lower || value > upper;
            Ok(json!({
                "anomaly": anomaly,
                "method": "iqr",
                "q1": q1,
                "q3": q3,
                "iqr": iqr,
                "lower": lower,
                "upper": upper
            }))
        }
        _ => Err(format!(
            "is_anomaly: unknown method '{}'. Use 'zscore' or 'iqr'.",
            method
        )),
    }
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("is_anomaly", is_anomaly)]
}
