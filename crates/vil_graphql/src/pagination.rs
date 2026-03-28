// =============================================================================
// VIL GraphQL — Pagination (Relay Connection Spec)
// =============================================================================

use serde::Serialize;

/// Relay-style page info.
#[derive(Debug, Clone, Serialize)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub total_count: u64,
    pub page: usize,
    pub page_size: usize,
}

/// Paginated result.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResult<T: Serialize> {
    pub items: Vec<T>,
    pub page_info: PageInfo,
}

/// Calculate pagination parameters.
pub fn calc_pagination(
    limit: Option<usize>,
    offset: Option<usize>,
    default_size: usize,
    max_size: usize,
) -> (usize, usize) {
    let size = limit.unwrap_or(default_size).min(max_size);
    let off = offset.unwrap_or(0);
    (size, off)
}
