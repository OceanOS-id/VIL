//! Trigger dispatch — start non-webhook triggers at startup.
//!
//! Webhook: handled by HttpSink in pipeline mode.
//! Cron: delegates to vil_trigger_cron for standard cron expressions,
//!        or uses simple interval for shorthand ("5s", "2m", "1h").
//! Other triggers: feature-gated, wire to VIL trigger crates.

use crate::graph::VilwGraph;
use crate::executor::{self, ExecConfig};
use std::sync::Arc;

/// Trigger handle — can be stopped.
pub struct TriggerHandle {
    pub trigger_type: String,
    pub cancel: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TriggerHandle {
    pub fn stop(self) {
        if let Some(tx) = self.cancel { let _ = tx.send(()); }
    }
}

/// Start all non-webhook triggers for a workflow graph.
#[allow(unused_mut, unused_variables)]
pub async fn start_triggers(
    graph: Arc<VilwGraph>,
    config: Arc<ExecConfig>,
) -> Vec<TriggerHandle> {
    let mut handles = Vec::new();

    match graph.trigger_type.as_str() {
        "webhook" => {
            // Handled by HttpSink pipeline — nothing here
        }

        "cron" => {
            let trigger_node = &graph.nodes[graph.entry_node];
            let expression: String = trigger_node.config.get("cron")
                .and_then(|c| c.get("expression"))
                .and_then(|v| v.as_str())
                .or_else(|| trigger_node.config.get("expression").and_then(|v| v.as_str()))
                .unwrap_or("60s")
                .to_string();

            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();

            // Try standard cron format via vil_trigger_cron
            #[cfg(feature = "triggers")]
            if is_cron_expression(&expression) {
                let leaked: &'static str = Box::leak(expression.clone().into_boxed_str());
                let cron_config = vil_trigger_cron::CronConfig::new(0, leaked);
                match vil_trigger_cron::create_cron_trigger(cron_config) {
                    Ok((trigger, mut rx)) => {
                        let g = graph.clone();
                        let c = config.clone();
                        tokio::spawn(async move {
                            use vil_trigger_core::TriggerSource;
                            let _ = trigger.start(Arc::new(|_| {})).await;
                            loop {
                                tokio::select! {
                                    Some(event) = rx.recv() => {
                                        let input = serde_json::json!({
                                            "_trigger": "cron",
                                            "_schedule": expression,
                                            "_fired_at": event.timestamp_ns,
                                            "_sequence": event.sequence,
                                        });
                                        if let Err(e) = executor::execute(&g, input, &c).await {
                                            tracing::warn!("cron trigger error: {}", e);
                                        }
                                    }
                                    _ = &mut cancel_rx => {
                                        tracing::info!("cron trigger stopped for {}", g.id);
                                        break;
                                    }
                                }
                            }
                        });

                        handles.push(TriggerHandle {
                            trigger_type: "cron".into(),
                            cancel: Some(cancel_tx),
                        });
                        return handles;
                    }
                    Err(e) => {
                        tracing::warn!("vil_trigger_cron parse failed ({}), falling back to interval: {}", expression, e);
                    }
                }
            }

            // Fallback: simple interval ("5s", "2m", "1h")
            let interval = parse_interval(&expression);
            let g = graph.clone();
            let c = config.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(interval);
                loop {
                    tokio::select! {
                        _ = tick.tick() => {
                            let input = serde_json::json!({
                                "_trigger": "cron",
                                "_schedule": expression,
                                "_fired_at": now_epoch_secs(),
                            });
                            if let Err(e) = executor::execute(&g, input, &c).await {
                                tracing::warn!("cron trigger error: {}", e);
                            }
                        }
                        _ = &mut cancel_rx => {
                            tracing::info!("cron trigger stopped for {}", g.id);
                            break;
                        }
                    }
                }
            });

            handles.push(TriggerHandle {
                trigger_type: "cron".into(),
                cancel: Some(cancel_tx),
            });
        }

        #[cfg(feature = "triggers")]
        "mqtt" | "iot" => {
            let trigger_source = vil_trigger_iot::process::create_trigger(vil_trigger_iot::IotConfig::new("localhost", 1883, "vil/#", "vil-vwfd-iot"));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            let c = config.clone();
            tokio::spawn(async move {
                let _ = trigger_source.start(Arc::new(|_| {})).await;
                tracing::info!("IoT/MQTT trigger active for {}", g.id);
                let _ = cancel_rx.await;
            });
            handles.push(TriggerHandle { trigger_type: "iot".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "cdc" => {
            let trigger_source = vil_trigger_cdc::process::create_trigger(vil_trigger_cdc::CdcConfig::new("host=localhost dbname=vil", "vil_cdc_slot", "vil_pub"));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                let _ = trigger_source.start(Arc::new(|_| {})).await;
                tracing::info!("CDC trigger active for {}", g.id);
                let _ = cancel_rx.await;
            });
            handles.push(TriggerHandle { trigger_type: "cdc".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "fs" => {
            let (trigger, mut rx) = vil_trigger_fs::create_fs_trigger(vil_trigger_fs::FsConfig::new(0, "/tmp"));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            let c = config.clone();
            tokio::spawn(async move {
                use vil_trigger_core::TriggerSource;
                let _ = trigger.start(Arc::new(|_| {})).await;
                loop {
                    tokio::select! {
                        Some(event) = rx.recv() => {
                            let input = serde_json::json!({
                                "_trigger": "fs",
                                "_event_kind_hash": event.kind_hash,
                                "_timestamp": event.timestamp_ns,
                            });
                            if let Err(e) = executor::execute(&g, input, &c).await {
                                tracing::warn!("fs trigger error: {}", e);
                            }
                        }
                        _ = &mut cancel_rx => break,
                    }
                }
            });
            handles.push(TriggerHandle { trigger_type: "fs".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "email" => {
            let trigger_source = vil_trigger_email::process::create_trigger(vil_trigger_email::EmailConfig::new("imap.localhost", 993, "user@localhost", "password", "INBOX"));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                let _ = trigger_source.start(Arc::new(|_| {})).await;
                tracing::info!("Email trigger active for {}", g.id);
                let _ = cancel_rx.await;
            });
            handles.push(TriggerHandle { trigger_type: "email".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "evm" => {
            let trigger_source = vil_trigger_evm::process::create_trigger(vil_trigger_evm::EvmConfig::new("wss://localhost:8546", "0x0000000000000000000000000000000000000000", "Transfer(address,address,uint256)"));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                let _ = trigger_source.start(Arc::new(|_| {})).await;
                tracing::info!("EVM trigger active for {}", g.id);
                let _ = cancel_rx.await;
            });
            handles.push(TriggerHandle { trigger_type: "evm".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "kafka" => {
            let trigger_node = &graph.nodes[graph.entry_node];
            let brokers = trigger_node.config.get("kafka").and_then(|c| c.get("brokers")).and_then(|v| v.as_str()).unwrap_or("localhost:9092");
            let topic = trigger_node.config.get("kafka").and_then(|c| c.get("topic")).and_then(|v| v.as_str()).unwrap_or("default");
            let group = trigger_node.config.get("kafka").and_then(|c| c.get("group_id")).and_then(|v| v.as_str()).unwrap_or("vil-vwfd");
            let trigger = vil_trigger_kafka::create_trigger(vil_trigger_kafka::KafkaConfig::new(brokers, topic, group));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            let c = config.clone();
            tokio::spawn(async move {
                use vil_trigger_core::TriggerSource;
                let _ = trigger.start(Arc::new(move |event| {
                    tracing::info!("kafka trigger event: seq={}", event.sequence);
                })).await;
                let _ = cancel_rx.await;
            });
            handles.push(TriggerHandle { trigger_type: "kafka".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "s3" | "s3_event" => {
            let trigger_node = &graph.nodes[graph.entry_node];
            let bucket = trigger_node.config.get("s3").and_then(|c| c.get("bucket")).and_then(|v| v.as_str()).unwrap_or("default");
            let prefix = trigger_node.config.get("s3").and_then(|c| c.get("prefix")).and_then(|v| v.as_str()).unwrap_or("");
            let trigger = vil_trigger_s3::create_trigger(vil_trigger_s3::S3Config::new(bucket, prefix, 30));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                use vil_trigger_core::TriggerSource;
                let _ = trigger.start(Arc::new(|_| {})).await;
                tracing::info!("S3 trigger active for {}", g.id);
            });
            handles.push(TriggerHandle { trigger_type: "s3".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "sftp" => {
            let trigger_node = &graph.nodes[graph.entry_node];
            let host = trigger_node.config.get("sftp").and_then(|c| c.get("host")).and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = trigger_node.config.get("sftp").and_then(|c| c.get("port")).and_then(|v| v.as_u64()).unwrap_or(22) as u16;
            let user = trigger_node.config.get("sftp").and_then(|c| c.get("username")).and_then(|v| v.as_str()).unwrap_or("user");
            let pass = trigger_node.config.get("sftp").and_then(|c| c.get("password")).and_then(|v| v.as_str()).unwrap_or("");
            let dir = trigger_node.config.get("sftp").and_then(|c| c.get("remote_dir")).and_then(|v| v.as_str()).unwrap_or("/upload");
            let trigger = vil_trigger_sftp::create_trigger(vil_trigger_sftp::SftpConfig::new(host, port, user, pass, dir, 60));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                use vil_trigger_core::TriggerSource;
                let _ = trigger.start(Arc::new(|_| {})).await;
                tracing::info!("SFTP trigger active for {}", g.id);
            });
            handles.push(TriggerHandle { trigger_type: "sftp".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "db_poll" | "poll" => {
            let trigger_node = &graph.nodes[graph.entry_node];
            let db_url = trigger_node.config.get("db_poll").and_then(|c| c.get("database_url")).and_then(|v| v.as_str()).unwrap_or("sqlite::memory:");
            let table = trigger_node.config.get("db_poll").and_then(|c| c.get("table")).and_then(|v| v.as_str()).unwrap_or("events");
            let id_col = trigger_node.config.get("db_poll").and_then(|c| c.get("id_column")).and_then(|v| v.as_str()).unwrap_or("id");
            let trigger = vil_trigger_db_poll::create_trigger(vil_trigger_db_poll::DbPollConfig::new(db_url, table, id_col, 10));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                use vil_trigger_core::TriggerSource;
                let _ = trigger.start(Arc::new(|_| {})).await;
                tracing::info!("DB poll trigger active for {}", g.id);
            });
            handles.push(TriggerHandle { trigger_type: "db_poll".into(), cancel: Some(cancel_tx) });
        }
        #[cfg(feature = "triggers")]
        "grpc" | "grpc_stream" => {
            let trigger_node = &graph.nodes[graph.entry_node];
            let endpoint = trigger_node.config.get("grpc").and_then(|c| c.get("endpoint")).and_then(|v| v.as_str()).unwrap_or("http://localhost:50051");
            let service = trigger_node.config.get("grpc").and_then(|c| c.get("service")).and_then(|v| v.as_str()).unwrap_or("events");
            let method = trigger_node.config.get("grpc").and_then(|c| c.get("method")).and_then(|v| v.as_str()).unwrap_or("stream");
            let trigger = vil_trigger_grpc::create_trigger(vil_trigger_grpc::GrpcConfig::new(endpoint, service, method));
            let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
            let g = graph.clone();
            tokio::spawn(async move {
                use vil_trigger_core::TriggerSource;
                let _ = trigger.start(Arc::new(|_| {})).await;
                tracing::info!("gRPC trigger active for {}", g.id);
            });
            handles.push(TriggerHandle { trigger_type: "grpc".into(), cancel: Some(cancel_tx) });
        }
        other => {
            tracing::warn!("trigger type '{}' not supported", other);
        }
    }

    handles
}

/// Check if expression looks like standard cron format (contains spaces + field count).
#[cfg(feature = "triggers")]
fn is_cron_expression(expr: &str) -> bool {
    let parts: Vec<&str> = expr.trim().split_whitespace().collect();
    parts.len() >= 5 // 5-field or 6-field cron
}

/// Parse simple interval shorthand → Duration.
fn parse_interval(schedule: &str) -> std::time::Duration {
    let s = schedule.trim();
    if s.ends_with("ms") {
        std::time::Duration::from_millis(s[..s.len()-2].parse().unwrap_or(60_000))
    } else if s.ends_with('s') {
        std::time::Duration::from_secs(s[..s.len()-1].parse().unwrap_or(60))
    } else if s.ends_with('m') {
        std::time::Duration::from_secs(s[..s.len()-1].parse::<u64>().unwrap_or(1) * 60)
    } else if s.ends_with('h') {
        std::time::Duration::from_secs(s[..s.len()-1].parse::<u64>().unwrap_or(1) * 3600)
    } else if let Ok(secs) = s.parse::<u64>() {
        std::time::Duration::from_secs(secs)
    } else {
        std::time::Duration::from_secs(60)
    }
}

fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interval() {
        assert_eq!(parse_interval("5s"), std::time::Duration::from_secs(5));
        assert_eq!(parse_interval("2m"), std::time::Duration::from_secs(120));
        assert_eq!(parse_interval("1h"), std::time::Duration::from_secs(3600));
        assert_eq!(parse_interval("500ms"), std::time::Duration::from_millis(500));
        assert_eq!(parse_interval("30"), std::time::Duration::from_secs(30));
    }
}
