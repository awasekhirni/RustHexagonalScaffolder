//! GraphQL resolver for the `Users` context.
//!
//! Delegates to the existing application `ServicePort` so the
//! hexagonal boundary is preserved.

use async_graphql::{Context, FieldResult, Object, ID};
use tracing::instrument;

use uuid::Uuid;
use crate::application::dto::users_dto::CreateUsersInput;
use crate::application::ports::repository::UsersServicePort;
use crate::domain::entities::users::Users;
use crate::presentation::state::AppState;

#[derive(Debug, Clone, Copy)]
pub struct UsersQuery;
#[derive(Debug, Clone, Copy)]
pub struct UsersMutation;

impl Default for UsersQuery { fn default() -> Self { Self } }
impl Default for UsersMutation { fn default() -> Self { Self } }

#[Object]
impl UsersQuery {
    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::users::list"))]
    async fn users_list(
        &self,
        ctx: &Context<'_>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> FieldResult<Vec<Users>> {
        let state = ctx.data::<AppState>()?;
        let p = crate::application::ports::repository::Page::new(
            page.unwrap_or(1), page_size.unwrap_or(50));
        let result = state.users_service.list(p).await?;
        Ok(result.items)
    }

    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::users::get"))]
    async fn users_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> FieldResult<Option<Users>> {
        let state = ctx.data::<AppState>()?;
        let parsed: Uuid = id.as_str().parse().map_err(|e| async_graphql::Error::new(format!("invalid id: {e}")))?;
        Ok(state.users_service.get(parsed).await?)
    }
}

#[Object]
impl UsersMutation {
    #[instrument(skip(ctx, input), fields(category = "runtime", function_id = "graphql::users::create"))]
    async fn users_create(
        &self,
        ctx: &Context<'_>,
        input: serde_json::Value,
    ) -> FieldResult<Users> {
        let state = ctx.data::<AppState>()?;
        let input: CreateUsersInput = serde_json::from_value(input)
            .map_err(|e| async_graphql::Error::new(format!("invalid input: {e}")))?;
        Ok(state.users_service.create(input).await?)
    }

    #[instrument(skip(ctx), fields(category = "runtime", function_id = "graphql::users::delete"))]
    async fn users_delete(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> FieldResult<bool> {
        let state = ctx.data::<AppState>()?;
        let parsed: Uuid = id.as_str().parse().map_err(|e| async_graphql::Error::new(format!("invalid id: {e}")))?;
        Ok(state.users_service.delete(parsed).await?)
    }
}
