use crate::client::NatsClient;

pub async fn check_health(client: &NatsClient) -> (bool, String) {
    if client.is_connected() {
        (true, "connected".into())
    } else {
        (false, "disconnected".into())
    }
}
