//! Request/response DTOs for the `ProductCategories` context.
//!
//! `CreateProductCategoriesInput` covers every client-writable column of an INSERT
//! (server-generated columns are filled by the database); `UpdateProductCategoriesInput`
//! is fully optional so `PUT` behaves as a partial update.

use serde::Deserialize;
use utoipa::ToSchema;

/// Payload accepted by `POST /product_categories` — validated before persistence.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateProductCategoriesInput {
    /// Column `name`.
    pub name: String,
    /// Column `type`.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Column `status`.
    pub status: String,
}

impl CreateProductCategoriesInput {
    /// Bean-style validation invoked by the application service.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name must not be empty".to_string());
        }
        if self.r#type.trim().is_empty() {
            return Err("type must not be empty".to_string());
        }
        if self.status.trim().is_empty() {
            return Err("status must not be empty".to_string());
        }
        Ok(())
    }
}

/// Payload accepted by `PUT /product_categories/{id}` — every field optional.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateProductCategoriesInput {
    /// Optional new value for `name`.
    pub name: Option<String>,
    /// Optional new value for `type`.
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    /// Optional new value for `status`.
    pub status: Option<String>,
}
