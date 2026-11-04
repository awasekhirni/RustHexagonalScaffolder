//! Mapper for `product_categories` — translates input models into domain entities.
//!
//! Columns the database generates are left at their default here; the
//! repository INSERT simply omits them and `RETURNING *` hands the real
//! values back.

use crate::domain::product_categories::entity::ProductCategories;
use crate::domain::product_categories::input::{CreateProductCategoriesInput, UpdateProductCategoriesInput};

/// Input ↔ entity translation for `product_categories`.
pub struct ProductCategoriesMapper;

impl ProductCategoriesMapper {
    /// Builds a fresh entity from a validated create payload.
    pub fn to_entity(input: CreateProductCategoriesInput) -> ProductCategories {
        ProductCategories {
            name: input.name,
            r#type: input.r#type,
            status: input.status,
            ..Default::default()
        }
    }

    /// Applies a partial update onto the loaded entity — only fields
    /// present in the payload are overwritten.
    pub fn apply_update(mut current: ProductCategories, input: &UpdateProductCategoriesInput) -> ProductCategories {
        if let Some(v) = &input.name {
            current.name = v.clone();
        }
        if let Some(v) = &input.r#type {
            current.r#type = v.clone();
        }
        if let Some(v) = &input.status {
            current.status = v.clone();
        }
        current
    }
}
