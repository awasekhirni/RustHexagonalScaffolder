//! Offset pagination query parameters and the envelope returned by list
//! endpoints.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Query parameters for offset pagination.
#[derive(Debug, Clone, Deserialize, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    /// 1-based page number (default 1).
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size, clamped to 1..=200 (default 20).
    #[validate(range(min = 1, max = 200))]
    pub page_size: Option<u32>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

impl Pagination {
    /// Returns the page number (1-based).
    #[must_use]
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }
    /// Returns the page size, clamped to 1..=200.
    #[must_use]
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).clamp(1, 200)
    }
    /// SQL OFFSET.
    #[must_use]
    pub fn offset(&self) -> i64 {
        ((self.page().saturating_sub(1)) as i64) * (self.page_size() as i64)
    }
    /// SQL LIMIT.
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.page_size() as i64
    }
    /// Total pages for a given row count.
    #[must_use]
    pub fn total_pages(&self, total: i64) -> i64 {
        let ps = self.page_size() as i64;
        if ps == 0 {
            return 0;
        }
        (total + ps - 1) / ps
    }
}

/// Envelope returned by list endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

impl<T> PageResponse<T> {
    /// Builds a page response from the items + total.
    #[must_use]
    pub fn new(items: Vec<T>, total: i64, page: u32, page_size: u32) -> Self {
        Self {
            items,
            total,
            page,
            page_size,
        }
    }
}
