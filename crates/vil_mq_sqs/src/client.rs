// =============================================================================
// vil_mq_sqs::client — SqsClient (send, receive, delete) + SnsClient (publish)
// =============================================================================

use crate::config::SqsConfig;
use crate::error::SqsFault;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_sns::Client as AwsSnsClient;
use aws_sdk_sqs::Client as AwsSqsClient;
use vil_log::dict::register_str;

/// A received SQS message.
#[derive(Debug, Clone)]
pub struct SqsMessage {
    /// Message body bytes.
    pub body: Vec<u8>,
    /// Receipt handle (used for deletion).
    pub receipt_handle: String,
    /// Hash of the queue URL this message came from.
    pub queue_hash: u32,
    /// Approximate receive count.
    pub receive_count: u32,
}

/// SQS client backed by the AWS SDK.
pub struct SqsClient {
    inner: AwsSqsClient,
    config: SqsConfig,
}

impl SqsClient {
    /// Build an SqsClient from config, loading AWS credentials from environment.
    pub async fn from_config(config: SqsConfig) -> Result<Self, SqsFault> {
        let region_hash = register_str(&config.region);
        let region = Region::new(config.region.clone());

        let mut cfg_builder = aws_config::defaults(BehaviorVersion::latest()).region(region);

        if let Some(ref endpoint) = config.endpoint {
            cfg_builder = cfg_builder.endpoint_url(endpoint);
        }

        let aws_cfg = cfg_builder.load().await;

        let inner = AwsSqsClient::new(&aws_cfg);

        let _ = region_hash; // used above
        Ok(Self { inner, config })
    }

    /// Send a message to the configured SQS queue.
    pub async fn send_message(&self, body: &[u8]) -> Result<(), SqsFault> {
        let __start = std::time::Instant::now();
        let queue_hash = register_str(&self.config.queue_url);

        let body_str = std::str::from_utf8(body).map_err(|_| SqsFault::InvalidMessage)?;

        self.inner
            .send_message()
            .queue_url(&self.config.queue_url)
            .message_body(body_str)
            .send()
            .await
            .map_err(|_| SqsFault::SendFailed {
                queue_hash,
                error_code: 0,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("sqs"),
                    topic_hash: queue_hash,
                    message_bytes: body.len() as u32,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 0, // publish
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    /// Receive up to `config.max_messages` messages from the queue.
    pub async fn receive_messages(&self) -> Result<Vec<SqsMessage>, SqsFault> {
        let __start = std::time::Instant::now();
        let queue_hash = register_str(&self.config.queue_url);

        let resp = self
            .inner
            .receive_message()
            .queue_url(&self.config.queue_url)
            .max_number_of_messages(self.config.max_messages)
            .visibility_timeout(self.config.visibility_timeout_secs)
            .wait_time_seconds(self.config.wait_time_secs)
            .send()
            .await
            .map_err(|_| SqsFault::ReceiveFailed { queue_hash })?;

        let messages = resp.messages.unwrap_or_default();
        let count = messages.len() as u32;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("sqs"),
                    topic_hash: queue_hash,
                    message_bytes: count,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 1, // consume
                    ..Default::default()
                }
            );
        }

        let result = messages
            .into_iter()
            .filter_map(|m| {
                let body = m.body.as_deref()?.as_bytes().to_vec();
                let receipt_handle = m.receipt_handle?;
                let receive_count = m
                    .attributes
                    .as_ref()
                    .and_then(|a| {
                        use aws_sdk_sqs::types::MessageSystemAttributeName;
                        a.get(&MessageSystemAttributeName::ApproximateReceiveCount)
                            .and_then(|v| v.parse().ok())
                    })
                    .unwrap_or(0);
                Some(SqsMessage {
                    body,
                    receipt_handle,
                    queue_hash,
                    receive_count,
                })
            })
            .collect();

        Ok(result)
    }

    /// Delete a message from the queue after successful processing.
    pub async fn delete_message(&self, receipt_handle: &str) -> Result<(), SqsFault> {
        let __start = std::time::Instant::now();
        let queue_hash = register_str(&self.config.queue_url);

        self.inner
            .delete_message()
            .queue_url(&self.config.queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .map_err(|_| SqsFault::DeleteFailed {
                queue_hash,
                error_code: 0,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("sqs"),
                    topic_hash: queue_hash,
                    message_bytes: 0,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 2, // ack (delete = ack in SQS)
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    pub fn config(&self) -> &SqsConfig {
        &self.config
    }
}

/// SNS client for publishing to topics.
pub struct SnsClient {
    inner: AwsSnsClient,
}

impl SnsClient {
    /// Build an SnsClient from the same config region/endpoint as SQS.
    pub async fn from_config(config: &SqsConfig) -> Result<Self, SqsFault> {
        let region_hash = register_str(&config.region);
        let region = Region::new(config.region.clone());

        let mut cfg_builder = aws_config::defaults(BehaviorVersion::latest()).region(region);

        if let Some(ref endpoint) = config.endpoint {
            cfg_builder = cfg_builder.endpoint_url(endpoint);
        }

        let aws_cfg = cfg_builder.load().await;
        let _ = region_hash;

        Ok(Self {
            inner: AwsSnsClient::new(&aws_cfg),
        })
    }

    /// Publish a message to an SNS topic ARN.
    pub async fn publish(&self, topic_arn: &str, message: &[u8]) -> Result<(), SqsFault> {
        let __start = std::time::Instant::now();
        let topic_hash = register_str(topic_arn);

        let msg_str = std::str::from_utf8(message).map_err(|_| SqsFault::InvalidMessage)?;

        self.inner
            .publish()
            .topic_arn(topic_arn)
            .message(msg_str)
            .send()
            .await
            .map_err(|_| SqsFault::SnsPublishFailed {
                topic_hash,
                error_code: 0,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("sns"),
                    topic_hash,
                    message_bytes: message.len() as u32,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 0, // publish
                    ..Default::default()
                }
            );
        }

        Ok(())
    }
}
