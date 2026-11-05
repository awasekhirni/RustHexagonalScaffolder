//! API surface — OpenAPI 3.1 document + Swagger UI binding.
//!
//! The OpenAPI document is built by `utoipa` from the `#[utoipa::path]`
//! attributes on every handler, then upgraded to 3.1 at serve-time by
//! `openapi::to_3_1()`. The serialised form is served at
//! `/api-docs/openapi.json` and rendered by Swagger UI at `/swagger-ui`.

pub mod openapi;

pub use openapi::ApiDoc;
pub use openapi::serve_openapi_3_1;
