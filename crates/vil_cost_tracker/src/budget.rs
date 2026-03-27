use serde::{Deserialize, Serialize};
use std::fmt;

/// Budget period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetPeriod {
    Daily,
    Monthly,
    Total,
}

/// A spending budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub key: String,
    pub limit_usd: f64,
    pub period: BudgetPeriod,
}

impl Budget {
    pub fn new(key: impl Into<String>, limit_usd: f64, period: BudgetPeriod) -> Self {
        Self {
            key: key.into(),
            limit_usd,
            period,
        }
    }
}

/// Error returned when a budget is exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetExceeded {
    pub key: String,
    pub limit_usd: f64,
    pub current_usd: f64,
}

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Budget '{}' exceeded: limit ${:.4}, current ${:.4}",
            self.key, self.limit_usd, self.current_usd
        )
    }
}

impl std::error::Error for BudgetExceeded {}
