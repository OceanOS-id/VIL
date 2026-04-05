// =============================================================================
// example-807-trigger-evm-blockchain — DeFi Smart Contract Event Watcher
// =============================================================================
//
// Domain:   Decentralized Finance (DeFi)
// Business: EVM blockchain trigger fires on smart contract events such as
//           Transfer, Swap, or Approval. Watches a contract address via
//           WebSocket eth_subscribe for real-time log streaming.
//
// Demonstrates:
//   - EvmConfig with RPC URL, contract address, and event signature
//   - EvmTrigger via vil_trigger_evm (tokio-tungstenite WS + JSON-RPC)
//   - TriggerSource::start() with an EventCallback + mpsc relay
//   - Receiving TriggerEvent descriptors from the mpsc Receiver
//   - mq_log! auto-emitted by vil_trigger_evm on every EVM log
//   - StdoutDrain::resolved() output
//
// The example collects 5 blockchain events then exits.
// Requires an Ethereum WebSocket RPC endpoint (e.g. Infura, Alchemy, local node).
//
// Environment variables:
//   EVM_RPC_URL          — WebSocket RPC URL   (default: wss://mainnet.infura.io/ws/v3/YOUR_KEY)
//   EVM_CONTRACT_ADDRESS — Contract to watch    (default: USDC on mainnet)
//   EVM_EVENT_SIGNATURE  — Event signature      (default: Transfer(address,address,uint256))
// =============================================================================

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};
use vil_trigger_evm::{EvmConfig, EvmTrigger};

/// Number of blockchain events to collect before stopping.
const EVENT_COUNT: u32 = 5;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let log_config = LogConfig {
        ring_slots: 4096,
        level: LogLevel::Info,
        batch_size: 64,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(log_config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-807-trigger-evm-blockchain");
    println!("  DeFi Smart Contract Event Watcher — EVM trigger (collects {} events then exits)", EVENT_COUNT);
    println!();

    // ── Build EvmConfig from env ──
    // Default: watch USDC contract on Ethereum mainnet for Transfer events
    let evm_cfg = EvmConfig::new(
        env_or("EVM_RPC_URL", "wss://mainnet.infura.io/ws/v3/YOUR_KEY"),
        env_or("EVM_CONTRACT_ADDRESS", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        env_or("EVM_EVENT_SIGNATURE", "Transfer(address,address,uint256)"),
    );

    println!("  RPC URL     : {}", evm_cfg.rpc_url);
    println!("  Contract    : {}", evm_cfg.contract_address);
    println!("  Event sig   : {}", evm_cfg.event_signature);
    println!();

    // ── Create the trigger ──
    let trigger = Arc::new(EvmTrigger::new(evm_cfg));

    // ── Wire up mpsc channel for downstream consumption ──
    let (tx, mut rx) = tokio::sync::mpsc::channel::<TriggerEvent>(128);

    // Fire counter shared with callback
    let fires = Arc::new(AtomicU32::new(0));
    let fires_cb = fires.clone();

    // Callback: prints each TriggerEvent and relays to mpsc
    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = fires_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!(
            "  FIRE #{n}  seq={}  ts_ns={}  payload={}B  kind_hash={:#010x}  [EVM log emitted]",
            event.sequence, event.timestamp_ns, event.payload_bytes, event.kind_hash,
        );
        let _ = tx.try_send(event);
    });

    // ── Start the trigger in a background task ──
    let trigger_bg = trigger.clone();
    tokio::spawn(async move {
        if let Err(e) = trigger_bg.start(on_event).await {
            println!("  Trigger stopped with fault: {:?}", e);
        }
    });

    println!(
        "  Waiting for {} blockchain events (eth_subscribe logs)...",
        EVENT_COUNT
    );
    println!();

    // ── Drain the mpsc receiver — process DeFi contract events ──
    let mut received = 0u32;
    while received < EVENT_COUNT {
        match tokio::time::timeout(std::time::Duration::from_secs(120), rx.recv()).await {
            Ok(Some(event)) => {
                received += 1;

                // Classify event type based on payload size
                let event_type = if event.payload_bytes > 256 {
                    "Swap/MultiTransfer"
                } else if event.payload_bytes > 0 {
                    "Transfer"
                } else {
                    "Approval"
                };

                println!(
                    "  RECV  seq={}  payload_bytes={}  op={}  type={}  [blockchain event #{}/{}]",
                    event.sequence, event.payload_bytes, event.op, event_type, received, EVENT_COUNT
                );
                if received >= EVENT_COUNT {
                    break;
                }
            }
            Ok(None) => {
                println!("  Channel closed");
                break;
            }
            Err(_) => {
                println!("  Timeout waiting for blockchain event (120s)");
                break;
            }
        }
    }

    // ── Stop the trigger ──
    if let Err(e) = trigger.stop().await {
        println!("  Stop fault: {:?}", e);
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!(
        "  Done. {} DeFi contract events collected. mq_log! entries emitted above.",
        received
    );
    println!();
}
