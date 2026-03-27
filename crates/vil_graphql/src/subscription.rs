// =============================================================================
// VIL GraphQL — Subscriptions via EventBus → WebSocket
// =============================================================================

use serde::Serialize;

/// Subscription event types.
#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionEvent {
    pub entity: String,
    pub operation: SubscriptionOp,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub enum SubscriptionOp {
    Created,
    Updated,
    Deleted,
}

/// Subscription topic builder.
pub fn entity_topic(entity: &str, op: &SubscriptionOp) -> String {
    let op_str = match op {
        SubscriptionOp::Created => "created",
        SubscriptionOp::Updated => "updated",
        SubscriptionOp::Deleted => "deleted",
    };
    format!("{}:{}", entity.to_lowercase(), op_str)
}

/// Subscription registry — tracks active subscriptions.
pub struct SubscriptionRegistry {
    topics: dashmap::DashMap<String, u64>, // topic → subscriber count
}

impl SubscriptionRegistry {
    pub fn new() -> Self {
        Self { topics: dashmap::DashMap::new() }
    }

    pub fn subscribe(&self, topic: &str) {
        self.topics.entry(topic.to_string()).and_modify(|c| *c += 1).or_insert(1);
    }

    pub fn unsubscribe(&self, topic: &str) {
        if let Some(mut count) = self.topics.get_mut(topic) {
            *count = count.saturating_sub(1);
        }
    }

    pub fn subscriber_count(&self, topic: &str) -> u64 {
        self.topics.get(topic).map(|c| *c).unwrap_or(0)
    }

    pub fn active_topics(&self) -> Vec<String> {
        self.topics.iter().filter(|e| *e.value() > 0).map(|e| e.key().clone()).collect()
    }
}

impl Default for SubscriptionRegistry {
    fn default() -> Self { Self::new() }
}
