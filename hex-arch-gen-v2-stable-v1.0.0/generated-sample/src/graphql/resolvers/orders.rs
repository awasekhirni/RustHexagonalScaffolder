//! GraphQL resolver for the `Orders` context.
//!
//! Delegates to the existing application `ServicePort` so the
//! hexagonal boundary is preserved.

use async_graphql::{Context, FieldResult, Object, ID};
use tracing::instrument;

use uuid::Uuid;
use crate::application::dto::orders_dto::CreateOrdersInput;
use crate::application::ports::repository::OrdersServicePort;
use crate::domain::entities::orders::Orders;
use crate::presentation::state::AppState;

#[derive(Debug, Clone, Copy)]
pub struct OrdersQuery;
#[derive(Debug, Clone, Copy)]
pub struct OrdersMutation;

impl Default for OrdersQuery { fn default() -> Self { Self } }
impl Default for OrdersMutation { fn default() -> Self { Self } }

#[Object]
impl OrdersQuery {
    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::orders::list"))]
    async fn orders_list(
        &self,
        ctx: &Context<'_>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> FieldResult<Vec<Orders>> {
        let state = ctx.data::<AppState>()?;
        let p = crate::application::ports::repository::Page::new(
            page.unwrap_or(1), page_size.unwrap_or(50));
        let result = state.orders_service.list(p).await?;
        Ok(result.items)
    }

    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::orders::get"))]
    async fn orders_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> FieldResult<Option<Orders>> {
        let state = ctx.data::<AppState>()?;
        let parsed: Uuid = id.as_str().parse().map_err(|e| async_graphql::Error::new(format!("invalid id: {e}")))?;
        Ok(state.orders_service.get(parsed).await?)
    }
}

#[Object]
impl OrdersMutation {
    #[instrument(skip(ctx, input), fields(category = "runtime", function_id = "graphql::orders::create"))]
    async fn orders_create(
        &self,
        ctx: &Context<'_>,
        input: serde_json::Value,
    ) -> FieldResult<Orders> {
        let state = ctx.data::<AppState>()?;
        let input: CreateOrdersInput = serde_json::from_value(input)
            .map_err(|e| async_graphql::Error::new(format!("invalid input: {e}")))?;
        Ok(state.orders_service.create(input).await?)
    }

    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::orders::delete"))]
    async fn orders_delete(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> FieldResult<bool> {
        let state = ctx.data::<AppState>()?;
        let parsed: Uuid = id.as_str().parse().map_err(|e| async_graphql::Error::new(format!("invalid id: {e}")))?;
        Ok(state.orders_service.delete(parsed).await?)
    }
}
