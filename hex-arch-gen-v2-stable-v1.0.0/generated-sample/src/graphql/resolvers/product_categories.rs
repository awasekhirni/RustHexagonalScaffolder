//! GraphQL resolver for the `ProductCategories` context.
//!
//! Delegates to the existing application `ServicePort` so the
//! hexagonal boundary is preserved.

use async_graphql::{Context, FieldResult, Object, ID};
use tracing::instrument;

use crate::application::dto::product_categories_dto::CreateProductCategoriesInput;
use crate::application::ports::repository::ProductCategoriesServicePort;
use crate::domain::entities::product_categories::ProductCategories;
use crate::presentation::state::AppState;

#[derive(Debug, Clone, Copy)]
pub struct ProductCategoriesQuery;
#[derive(Debug, Clone, Copy)]
pub struct ProductCategoriesMutation;

impl Default for ProductCategoriesQuery { fn default() -> Self { Self } }
impl Default for ProductCategoriesMutation { fn default() -> Self { Self } }

#[Object]
impl ProductCategoriesQuery {
    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::product_categories::list"))]
    async fn product_categories_list(
        &self,
        ctx: &Context<'_>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> FieldResult<Vec<ProductCategories>> {
        let state = ctx.data::<AppState>()?;
        let p = crate::application::ports::repository::Page::new(
            page.unwrap_or(1), page_size.unwrap_or(50));
        let result = state.product_categories_service.list(p).await?;
        Ok(result.items)
    }

    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::product_categories::get"))]
    async fn product_categories_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> FieldResult<Option<ProductCategories>> {
        let state = ctx.data::<AppState>()?;
        let parsed: i32 = id.as_str().parse().map_err(|e| async_graphql::Error::new(format!("invalid id: {e}")))?;
        Ok(state.product_categories_service.get(parsed).await?)
    }
}

#[Object]
impl ProductCategoriesMutation {
    #[instrument(skip(ctx, input), fields(category = "runtime", function_id = "graphql::product_categories::create"))]
    async fn product_categories_create(
        &self,
        ctx: &Context<'_>,
        input: serde_json::Value,
    ) -> FieldResult<ProductCategories> {
        let state = ctx.data::<AppState>()?;
        let input: CreateProductCategoriesInput = serde_json::from_value(input)
            .map_err(|e| async_graphql::Error::new(format!("invalid input: {e}")))?;
        Ok(state.product_categories_service.create(input).await?)
    }

    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::product_categories::delete"))]
    async fn product_categories_delete(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> FieldResult<bool> {
        let state = ctx.data::<AppState>()?;
        let parsed: i32 = id.as_str().parse().map_err(|e| async_graphql::Error::new(format!("invalid id: {e}")))?;
        Ok(state.product_categories_service.delete(parsed).await?)
    }
}
