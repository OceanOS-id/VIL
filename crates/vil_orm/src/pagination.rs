use serde::Serialize;

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize)]
pub struct VilPage<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

/// Pagination metadata.
#[derive(Debug, Clone, Serialize)]
pub struct Pagination {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub pages: i64,
}

impl Pagination {
    pub fn new(page: i64, per_page: i64, total: i64) -> Self {
        let pages = if per_page > 0 { (total + per_page - 1) / per_page } else { 0 };
        Self { page, per_page, total, pages }
    }
}

impl<T: Serialize> VilPage<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, total: i64) -> Self {
        Self {
            data,
            pagination: Pagination::new(page, per_page, total),
        }
    }
}
