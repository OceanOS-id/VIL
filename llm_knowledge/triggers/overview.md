# Triggers — TriggerSource Overview

VIL triggers allow pipelines and handlers to be activated by external events beyond HTTP requests.

## Quick Reference

- Trait: `TriggerSource`
- 8 trigger types
- Integrates with `vil_workflow!` and `VilApp` service endpoints

## TriggerSource Trait

```rust
pub trait TriggerSource: Send + Sync + 'static {
    type Output: Send + Serialize;

    async fn next(&mut self) -> Option<TriggerEvent<Self::Output>>;
    fn id(&self) -> &str;
    fn trigger_type(&self) -> TriggerType;
}
```

## All 8 Trigger Types

| Type | Struct | Activates on |
|------|--------|-------------|
| Cron | `CronTrigger` | Schedule (cron expression) |
| Filesystem | `FsTrigger` | File create/modify/delete |
| CDC | `CdcTrigger` | Database change data capture |
| Email | `EmailTrigger` | Incoming email (IMAP) |
| IoT | `IotTrigger` | MQTT/AMQP sensor messages |
| EVM | `EvmTrigger` | Ethereum/EVM smart contract events |
| Webhook | `WebhookTrigger` | Inbound HTTP webhook |
| Manual | `ManualTrigger` | Programmatic / test activation |

## Cron Trigger

```rust
use vil_trigger::CronTrigger;

let trigger = CronTrigger::new("0 */5 * * * *")  // every 5 minutes
    .id("cleanup-cron")
    .timezone("UTC")
    .build()?;

// Attach to pipeline
let (_ir, handles) = vil_workflow! {
    name: "ScheduledCleanup",
    trigger: trigger,
    instances: [ processor, sink ],
    routes: [ trigger.out -> processor.in (LoanWrite) ]
};
```

## Filesystem Trigger

```rust
use vil_trigger::{FsTrigger, FsEvent};

let trigger = FsTrigger::new("/data/incoming/")
    .id("file-watcher")
    .events(vec![FsEvent::Create, FsEvent::Modify])
    .glob("*.csv")
    .build()?;
```

Output payload:
```rust
FsTriggerEvent {
    path: PathBuf,
    event: FsEvent,   // Create | Modify | Delete | Rename
    size_bytes: Option<u64>,
}
```

## CDC Trigger (Change Data Capture)

```rust
use vil_trigger::{CdcTrigger, CdcConfig};

let trigger = CdcTrigger::new(CdcConfig {
    database_url: "postgres://user:pass@localhost/mydb".into(),
    slot_name: "vil_cdc_slot".into(),
    tables: vec!["public.orders".into(), "public.payments".into()],
    ..Default::default()
})
.id("orders-cdc")
.build().await?;
```

Output payload:
```rust
CdcEvent {
    table: String,
    operation: CdcOp,  // Insert | Update | Delete
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    lsn: u64,
}
```

## Email Trigger

```rust
use vil_trigger::{EmailTrigger, ImapConfig};

let trigger = EmailTrigger::new(ImapConfig {
    host: "imap.example.com".into(),
    port: 993,
    username: "inbox@example.com".into(),
    password: std::env::var("IMAP_PASS")?,
    folder: "INBOX".into(),
    ..Default::default()
})
.id("email-ingest")
.filter_subject("ORDER:")
.build().await?;
```

Output payload:
```rust
EmailEvent {
    from: String,
    subject: String,
    body_text: Option<String>,
    body_html: Option<String>,
    attachments: Vec<EmailAttachment>,
    received_at: DateTime<Utc>,
}
```

## IoT Trigger

```rust
use vil_trigger::{IotTrigger, IotConfig, IotProtocol};

let trigger = IotTrigger::new(IotConfig {
    protocol: IotProtocol::Mqtt {
        url: "mqtt://broker:1883".into(),
        topic: "sensors/#".into(),
        qos: 1,
    },
    ..Default::default()
})
.id("sensor-stream")
.build().await?;
```

## EVM Trigger (Blockchain)

```rust
use vil_trigger::{EvmTrigger, EvmConfig};

let trigger = EvmTrigger::new(EvmConfig {
    rpc_url: "wss://mainnet.infura.io/ws/v3/YOUR_KEY".into(),
    contract_address: "0xContractAddress".into(),
    event_signature: "Transfer(address,address,uint256)".into(),
    ..Default::default()
})
.id("erc20-transfers")
.build().await?;
```

Output payload:
```rust
EvmEvent {
    block_number: u64,
    tx_hash: String,
    log_index: u32,
    topics: Vec<String>,
    data: Bytes,
    decoded: Option<serde_json::Value>,
}
```

## Webhook Trigger

```rust
use vil_trigger::{WebhookTrigger};

// Spins up a lightweight HTTP listener for inbound webhooks
let trigger = WebhookTrigger::new()
    .id("stripe-webhook")
    .port(3099)
    .path("/webhook/stripe")
    .secret("stripe_signing_secret")    // HMAC-SHA256 validation
    .build()?;
```

## Manual Trigger (Testing)

```rust
use vil_trigger::ManualTrigger;

let (trigger, sender) = ManualTrigger::new("test-trigger");

// In test: fire the trigger
sender.send(json!({ "order_id": 42 })).await?;
```

## Attaching Triggers to VilApp

```rust
VilApp::new("event-processor")
    .port(8080)
    .trigger(cron_trigger, cron_handler)
    .trigger(fs_trigger, file_handler)
    .service(ServiceProcess::new("api")
        .endpoint(Method::GET, "/status", get(status)))
    .run().await;

#[vil_handler]
async fn cron_handler(event: TriggerEvent<CronTriggerEvent>) -> VilResponse<()> {
    app_log!(Info, "cron.fired", { id: event.trigger_id });
    run_cleanup().await?;
    VilResponse::ok(())
}
```

## Common Patterns

### CDC → pipeline → ClickHouse
```rust
let cdc = CdcTrigger::new(cdc_config).build().await?;
let ch_sink = ClickHouseSink::new(ch_pool).table("order_changes").build();

let (_ir, handles) = vil_workflow! {
    name: "CdcIngestion",
    trigger: cdc,
    instances: [ cdc, ch_sink ],
    routes: [ cdc.out -> ch_sink.in (LoanWrite) ]
};
```

### Cron → LLM report generation
```rust
let cron = CronTrigger::new("0 9 * * 1").build()?; // Monday 9am

VilApp::new("reporter")
    .trigger(cron, weekly_report_handler)
    .run().await;
```
