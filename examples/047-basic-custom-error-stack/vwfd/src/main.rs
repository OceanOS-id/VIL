// 047 — Core Banking Transfer with Error Handling (VWFD)
// Business logic identical to standard:
//   - 4 accounts: ACC-1001 (Alice 50M), ACC-1002 (Bob 25M), ACC-1003 (Charlie 75M frozen), ACC-1004 (Diana 100M)
//   - Transfer validation order: same account → frozen → limit → balance
//   - Error codes: INSUFFICIENT_FUNDS, ACCOUNT_FROZEN, TRANSACTION_LIMIT_EXCEEDED, ACCOUNT_NOT_FOUND
//   - Daily limit per account (ACC-1001/1002: 10M, ACC-1004: 20M)
use serde_json::{json, Value};
use std::sync::Mutex;
use std::collections::HashMap;

struct Account {
    holder_name: &'static str,
    balance_cents: i64,
    frozen: bool,
    daily_limit_cents: i64,
}

static ACCOUNTS: Mutex<Option<HashMap<String, Account>>> = Mutex::new(None);

fn init_accounts() -> HashMap<String, Account> {
    let mut m = HashMap::new();
    m.insert("ACC-1001".into(), Account { holder_name: "Alice Wijaya", balance_cents: 500_000_00, frozen: false, daily_limit_cents: 100_000_00 });
    m.insert("ACC-1002".into(), Account { holder_name: "Bob Santoso", balance_cents: 250_000_00, frozen: false, daily_limit_cents: 100_000_00 });
    m.insert("ACC-1003".into(), Account { holder_name: "Charlie Pratama", balance_cents: 750_000_00, frozen: true, daily_limit_cents: 100_000_00 });
    m.insert("ACC-1004".into(), Account { holder_name: "Diana Sari", balance_cents: 1_000_000_00, frozen: false, daily_limit_cents: 200_000_00 });
    m
}

fn get_accounts() -> HashMap<String, Account> {
    let mut lock = ACCOUNTS.lock().unwrap();
    if lock.is_none() { *lock = Some(init_accounts()); }
    // Return clone of current state (simplified — real would use RwLock)
    init_accounts()
}

fn list_accounts(_input: &Value) -> Result<Value, String> {
    let accounts = get_accounts();
    let mut list: Vec<Value> = accounts.iter().map(|(id, a)| {
        json!({"id": id, "holder_name": a.holder_name, "balance_cents": a.balance_cents, "frozen": a.frozen})
    }).collect();
    list.sort_by(|a, b| a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or("")));
    Ok(json!(list))
}

fn transfer(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let from = body["from_account"].as_str().unwrap_or("");
    let to = body["to_account"].as_str().unwrap_or("");
    let amount = body["amount_cents"].as_i64().unwrap_or(0);

    if amount <= 0 {
        return Ok(json!({"error": "amount_cents must be positive", "status": 400}));
    }
    if from == to {
        return Ok(json!({"error": "cannot transfer to the same account", "status": 400}));
    }

    let accounts = get_accounts();

    let from_acct = accounts.get(from)
        .ok_or_else(|| format!("ACCOUNT_NOT_FOUND: {}", from))?;
    let _to_acct = accounts.get(to)
        .ok_or_else(|| format!("ACCOUNT_NOT_FOUND: {}", to))?;

    if from_acct.frozen {
        return Ok(json!({"error": "ACCOUNT_FROZEN", "account_id": from, "status": 403}));
    }
    if _to_acct.frozen {
        return Ok(json!({"error": "ACCOUNT_FROZEN", "account_id": to, "status": 403}));
    }
    if amount > from_acct.daily_limit_cents {
        return Ok(json!({
            "error": "TRANSACTION_LIMIT_EXCEEDED",
            "account_id": from, "limit_cents": from_acct.daily_limit_cents,
            "requested_cents": amount, "status": 422
        }));
    }
    if from_acct.balance_cents < amount {
        return Ok(json!({
            "error": "INSUFFICIENT_FUNDS",
            "account_id": from, "available_cents": from_acct.balance_cents,
            "requested_cents": amount, "status": 422
        }));
    }

    let transfer_id = format!("TXN-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());

    Ok(json!({
        "transfer_id": transfer_id,
        "from_account": from, "to_account": to,
        "amount_cents": amount,
        "from_new_balance": from_acct.balance_cents - amount,
        "to_new_balance": _to_acct.balance_cents + amount,
        "status": "completed"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/047-basic-custom-error-stack/vwfd/workflows", 8080)
        .native("list_accounts", list_accounts)
        .native("transfer_handler", transfer)
        .run().await;
}
