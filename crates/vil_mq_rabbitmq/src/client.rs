// =============================================================================
// vil_mq_rabbitmq::client — RabbitClient: connect, publish, consume, ack
// =============================================================================

use crate::config::RabbitConfig;
use crate::error::RabbitFault;
use bytes::Bytes;
use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions,
        BasicQosOptions, QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use tokio::sync::mpsc;
use vil_log::dict::register_str;

/// A received message from RabbitMQ.
#[derive(Debug)]
pub struct RabbitMessage {
    /// Raw payload bytes.
    pub payload: Bytes,
    /// Delivery tag used for ack/nack.
    pub delivery_tag: u64,
    /// Routing key the message was published with.
    pub routing_key_hash: u32,
    /// Exchange hash.
    pub exchange_hash: u32,
}

/// RabbitMQ client wrapping a lapin connection.
///
/// This crate spawns 1 internal consumer task per consume() call.
/// Add 1 per active consumer to your `LogConfig.threads` for optimal ring sizing.
pub struct RabbitClient {
    connection: Connection,
    channel: Channel,
    config: RabbitConfig,
}

impl RabbitClient {
    /// Connect to RabbitMQ and open a channel with configured QoS.
    pub async fn connect(config: RabbitConfig) -> Result<Self, RabbitFault> {
        let uri_hash = register_str(&config.uri);
        let start = std::time::Instant::now();

        let conn = Connection::connect(&config.uri, ConnectionProperties::default())
            .await
            .map_err(|_| RabbitFault::ConnectionFailed {
                uri_hash,
                elapsed_ms: start.elapsed().as_millis() as u32,
            })?;

        let channel = conn
            .create_channel()
            .await
            .map_err(|_| RabbitFault::ChannelFailed { code: 0 })?;

        channel
            .basic_qos(config.prefetch_count, BasicQosOptions::default())
            .await
            .map_err(|_| RabbitFault::ChannelFailed { code: 1 })?;

        Ok(Self {
            connection: conn,
            channel,
            config,
        })
    }

    /// Publish a message to the given exchange with a routing key.
    pub async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
    ) -> Result<(), RabbitFault> {
        let __start = std::time::Instant::now();
        let exchange_hash = register_str(exchange);
        let routing_key_hash = register_str(routing_key);

        let result = self
            .channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default(),
            )
            .await
            .map_err(|_| RabbitFault::PublishFailed {
                exchange_hash,
                routing_key_hash,
            })?;

        result.await.map_err(|_| RabbitFault::PublishFailed {
            exchange_hash,
            routing_key_hash,
        })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("rabbitmq"),
                    topic_hash: exchange_hash,
                    message_bytes: payload.len() as u32,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 0, // publish
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    /// Start consuming messages from the given queue.
    /// Returns an mpsc receiver. Each received item is a `RabbitMessage`.
    pub async fn consume(&self, queue: &str) -> Result<mpsc::Receiver<RabbitMessage>, RabbitFault> {
        let queue_hash = register_str(queue);

        // Declare the queue (idempotent).
        self.channel
            .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|_| RabbitFault::ConsumeFailed { queue_hash })?;

        let consumer = self
            .channel
            .basic_consume(
                queue,
                &self.config.consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|_| RabbitFault::ConsumeFailed { queue_hash })?;

        let (tx, rx) = mpsc::channel(self.config.prefetch_count as usize);

        tokio::spawn(async move {
            let mut consumer = consumer;
            while let Some(delivery_result) = consumer.next().await {
                let Ok(delivery) = delivery_result else {
                    continue;
                };
                let __start = std::time::Instant::now();
                let payload_len = delivery.data.len() as u32;
                let rk_hash = register_str(delivery.routing_key.as_str());
                let ex_hash = register_str(delivery.exchange.as_str());

                let msg = RabbitMessage {
                    payload: Bytes::copy_from_slice(&delivery.data),
                    delivery_tag: delivery.delivery_tag,
                    routing_key_hash: rk_hash,
                    exchange_hash: ex_hash,
                };

                let __elapsed = __start.elapsed();
                {
                    use vil_log::{mq_log, types::MqPayload};
                    mq_log!(
                        Info,
                        MqPayload {
                            broker_hash: register_str("rabbitmq"),
                            topic_hash: queue_hash,
                            message_bytes: payload_len,
                            e2e_latency_us: __elapsed.as_micros() as u32,
                            op_type: 1, // consume
                            ..Default::default()
                        }
                    );
                }

                if tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    /// Acknowledge a message by delivery tag.
    pub async fn ack(&self, delivery_tag: u64) -> Result<(), RabbitFault> {
        let __start = std::time::Instant::now();

        self.channel
            .basic_ack(delivery_tag, BasicAckOptions::default())
            .await
            .map_err(|_| RabbitFault::AckFailed {
                delivery_tag: delivery_tag as u32,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("rabbitmq"),
                    topic_hash: 0,
                    message_bytes: 0,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 2, // ack
                    offset: delivery_tag,
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    /// Negative-acknowledge a message (optionally requeue).
    pub async fn nack(&self, delivery_tag: u64, requeue: bool) -> Result<(), RabbitFault> {
        let __start = std::time::Instant::now();

        self.channel
            .basic_nack(
                delivery_tag,
                BasicNackOptions {
                    requeue,
                    ..BasicNackOptions::default()
                },
            )
            .await
            .map_err(|_| RabbitFault::AckFailed {
                delivery_tag: delivery_tag as u32,
            })?;

        let __elapsed = __start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("rabbitmq"),
                    topic_hash: 0,
                    message_bytes: 0,
                    e2e_latency_us: __elapsed.as_micros() as u32,
                    op_type: 3, // nack
                    offset: delivery_tag,
                    ..Default::default()
                }
            );
        }

        Ok(())
    }

    /// Close the channel and connection gracefully.
    pub async fn close(&self) {
        let _ = self.channel.close(200u16, "normal shutdown").await;
        let _ = self.connection.close(200u16, "normal shutdown").await;
    }

    pub fn config(&self) -> &RabbitConfig {
        &self.config
    }
}
