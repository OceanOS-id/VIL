// ╔════════════════════════════════════════════════════════════╗
// ║  047 — Core Banking Error Handling Stack                  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Core Banking Error Handling           ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: #[derive(DeriveVilError)], #[vil_error(status)],║
// ║            HandlerResult, VilError, VilModel, ServiceCtx  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Transfer endpoint with structured domain errors ║
// ║  InsufficientFunds, AccountFrozen, TransactionLimitExceeded║
// ║  AccountNotFound. In-memory account store with RwLock.     ║
// ║                                                           ║
// ║  Endpoints:                                               ║
// ║    POST /api/banking/transfer     → transfer between accts║
// ║    GET  /api/banking/account/:id  → get account balance   ║
// ║    GET  /api/banking/accounts     → list all accounts     ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-custom-error-stack
// Test:
//   curl http://localhost:8080/api/banking/accounts
//   curl http://localhost:8080/api/banking/account/ACC-1001
//   curl -X POST http://localhost:8080/api/banking/transfer \
//     -H 'Content-Type: application/json' \
//     -d '{"from_account":"ACC-1001","to_account":"ACC-1002","amount_cents":50000}'
//   curl -X POST http://localhost:8080/api/banking/transfer \
//     -H 'Content-Type: application/json' \
//     -d '{"from_account":"ACC-1001","to_account":"ACC-1002","amount_cents":999999999}'

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use vil_server::prelude::*;

// ── Domain Errors ────────────────────────────────────────────────────────
// #[derive(DeriveVilError)] generates Display, Error, and From<Self> for VilError.
// Each variant maps to a specific HTTP status code via #[vil_error(status = N)].

#[derive(Debug, DeriveVilError)]
pub enum BankingError {
    /// Source account has insufficient balance for the transfer.
    #[vil_error(status = 422, code = "INSUFFICIENT_FUNDS")]
    InsufficientFunds {
        account_id: String,
        available_cents: i64,
        requested_cents: i64,
    },

    /// Account is frozen by compliance — no debits or credits allowed.
    #[vil_error(status = 403, code = "ACCOUNT_FROZEN")]
    AccountFrozen { account_id: String },

    /// Single transaction exceeds the configured daily limit.
    #[vil_error(status = 422, code = "TRANSACTION_LIMIT_EXCEEDED")]
    TransactionLimitExceeded {
        account_id: String,
        limit_cents: i64,
        requested_cents: i64,
    },

    /// Account does not exist in the system.
    #[vil_error(status = 404, code = "ACCOUNT_NOT_FOUND")]
    AccountNotFound { account_id: String },
}

// ── Domain Models ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Account {
    id: String,
    holder_name: String,
    balance_cents: i64,
    frozen: bool,
    daily_limit_cents: i64,
}

#[derive(Debug, Deserialize)]
struct TransferRequest {
    from_account: String,
    to_account: String,
    amount_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TransferResult {
    transfer_id: String,
    from_account: String,
    to_account: String,
    amount_cents: i64,
    from_new_balance: i64,
    to_new_balance: i64,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct AccountSummary {
    id: String,
    holder_name: String,
    balance_cents: i64,
    frozen: bool,
}

// ── State ────────────────────────────────────────────────────────────────

struct BankState {
    accounts: RwLock<HashMap<String, Account>>,
}

impl BankState {
    fn new() -> Self {
        let mut accounts = HashMap::new();

        accounts.insert("ACC-1001".into(), Account {
            id: "ACC-1001".into(),
            holder_name: "Alice Wijaya".into(),
            balance_cents: 500_000_00, // Rp 500,000.00
            frozen: false,
            daily_limit_cents: 100_000_00,
        });
        accounts.insert("ACC-1002".into(), Account {
            id: "ACC-1002".into(),
            holder_name: "Bob Santoso".into(),
            balance_cents: 250_000_00, // Rp 250,000.00
            frozen: false,
            daily_limit_cents: 100_000_00,
        });
        accounts.insert("ACC-1003".into(), Account {
            id: "ACC-1003".into(),
            holder_name: "Charlie Pratama".into(),
            balance_cents: 750_000_00, // Rp 750,000.00
            frozen: true, // Compliance hold
            daily_limit_cents: 100_000_00,
        });
        accounts.insert("ACC-1004".into(), Account {
            id: "ACC-1004".into(),
            holder_name: "Diana Sari".into(),
            balance_cents: 1_000_000_00, // Rp 1,000,000.00
            frozen: false,
            daily_limit_cents: 200_000_00,
        });

        Self {
            accounts: RwLock::new(accounts),
        }
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /transfer — transfer between accounts with structured domain errors.
async fn transfer_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TransferResult>> {
    let req: TransferRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected from_account, to_account, amount_cents"))?;

    if req.amount_cents <= 0 {
        return Err(VilError::bad_request("amount_cents must be positive"));
    }

    if req.from_account == req.to_account {
        return Err(VilError::bad_request("cannot transfer to the same account"));
    }

    let state = ctx.state::<Arc<BankState>>()
        .map_err(|_| VilError::internal("bank state not found"))?;

    let mut accounts = state.accounts.write().unwrap();

    // Validate source account exists
    let from_acct = accounts.get(&req.from_account)
        .ok_or_else(|| BankingError::AccountNotFound {
            account_id: req.from_account.clone(),
        })?
        .clone();

    // Validate destination account exists
    let _to_acct = accounts.get(&req.to_account)
        .ok_or_else(|| BankingError::AccountNotFound {
            account_id: req.to_account.clone(),
        })?
        .clone();

    // Check if source is frozen
    if from_acct.frozen {
        return Err(BankingError::AccountFrozen {
            account_id: req.from_account.clone(),
        }.into());
    }

    // Check if destination is frozen
    if _to_acct.frozen {
        return Err(BankingError::AccountFrozen {
            account_id: req.to_account.clone(),
        }.into());
    }

    // Check transaction limit
    if req.amount_cents > from_acct.daily_limit_cents {
        return Err(BankingError::TransactionLimitExceeded {
            account_id: req.from_account.clone(),
            limit_cents: from_acct.daily_limit_cents,
            requested_cents: req.amount_cents,
        }.into());
    }

    // Check sufficient funds
    if from_acct.balance_cents < req.amount_cents {
        return Err(BankingError::InsufficientFunds {
            account_id: req.from_account.clone(),
            available_cents: from_acct.balance_cents,
            requested_cents: req.amount_cents,
        }.into());
    }

    // Execute transfer
    let from = accounts.get_mut(&req.from_account).unwrap();
    from.balance_cents -= req.amount_cents;
    let from_new = from.balance_cents;

    let to = accounts.get_mut(&req.to_account).unwrap();
    to.balance_cents += req.amount_cents;
    let to_new = to.balance_cents;

    // Generate transfer ID
    let transfer_id = format!("TXN-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    Ok(VilResponse::ok(TransferResult {
        transfer_id,
        from_account: req.from_account,
        to_account: req.to_account,
        amount_cents: req.amount_cents,
        from_new_balance: from_new,
        to_new_balance: to_new,
        status: "completed".into(),
    }))
}

/// GET /account/:id — get account balance and details.
async fn get_account(
    ctx: ServiceCtx,
    Path(id): Path<String>,
) -> HandlerResult<VilResponse<Account>> {
    let state = ctx.state::<Arc<BankState>>()
        .map_err(|_| VilError::internal("bank state not found"))?;

    let accounts = state.accounts.read().unwrap();
    let account = accounts.get(&id)
        .ok_or_else(|| BankingError::AccountNotFound {
            account_id: id.clone(),
        })?
        .clone();

    Ok(VilResponse::ok(account))
}

/// GET /accounts — list all accounts (summary view).
async fn list_accounts(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<AccountSummary>>> {
    let state = ctx.state::<Arc<BankState>>()
        .map_err(|_| VilError::internal("bank state not found"))?;

    let accounts = state.accounts.read().unwrap();
    let mut summaries: Vec<AccountSummary> = accounts.values().map(|a| AccountSummary {
        id: a.id.clone(),
        holder_name: a.holder_name.clone(),
        balance_cents: a.balance_cents,
        frozen: a.frozen,
    }).collect();

    // Sort by account ID for stable output
    summaries.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(VilResponse::ok(summaries))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let state = Arc::new(BankState::new());

    let banking_svc = ServiceProcess::new("banking")
        .endpoint(Method::POST, "/transfer", post(transfer_handler))
        .endpoint(Method::GET, "/account/:id", get(get_account))
        .endpoint(Method::GET, "/accounts", get(list_accounts))
        .state(state);

    VilApp::new("core-banking-error-stack")
        .port(8080)
        .observer(true)
        .service(banking_svc)
        .run()
        .await;
}
