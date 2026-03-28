// =============================================================================
// vil_mq_pulsar::client — PulsarClient: connect to broker
// =============================================================================

use crate::config::PulsarConfig;
use crate::error::PulsarFault;
use pulsar::{Authentication, Pulsar, TokioExecutor};
use vil_log::dict::register_str;

/// Pulsar client connected to a broker.
///
/// Use `PulsarClient::connect` to create, then call `.producer()` or `.consumer()`
/// to obtain typed handles.
pub struct PulsarClient {
    pub(crate) inner: Pulsar<TokioExecutor>,
    pub(crate) config: PulsarConfig,
}

impl PulsarClient {
    /// Connect to the Pulsar broker specified in `config`.
    pub async fn connect(config: PulsarConfig) -> Result<Self, PulsarFault> {
        let __start = std::time::Instant::now();
        let url_hash = register_str(&config.url);

        let mut builder = Pulsar::builder(&config.url, TokioExecutor);

        if let Some(ref token) = config.auth_token {
            let auth = Authentication {
                name: "token".into(),
                data: token.as_bytes().to_vec(),
            };
            builder = builder.with_auth(auth);
        }

        let inner = builder
            .build()
            .await
            .map_err(|_| PulsarFault::ConnectionFailed {
                url_hash,
                elapsed_ms: __start.elapsed().as_millis() as u32,
            })?;

        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(
                Info,
                MqPayload {
                    broker_hash: register_str("pulsar"),
                    topic_hash: url_hash,
                    message_bytes: 0,
                    e2e_latency_us: __start.elapsed().as_micros() as u32,
                    op_type: 1, // consume (connection = setup for consume)
                    ..Default::default()
                }
            );
        }

        Ok(Self { inner, config })
    }

    pub fn config(&self) -> &PulsarConfig {
        &self.config
    }
}
