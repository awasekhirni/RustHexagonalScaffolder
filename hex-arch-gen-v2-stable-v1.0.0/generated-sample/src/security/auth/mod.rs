//! Authentication — JWT, refresh tokens, API keys, RBAC, extractor.
//!
//! v0.4.0 (closes G-H01, G-H02): real JWT issuer/verifier with key
//! rotation and refresh rotation; RBAC policy table with per-table
//! row-level authorization.

pub mod api_key;
pub mod dev_token;
pub mod extractor;
pub mod jwt;
pub mod rbac;
pub mod refresh;

pub use extractor::{AuthUser, ReqAuth};
pub use jwt::{Claims, JwtService, SharedJwtService};
pub use rbac::{RbacDecision, RbacEntry, RbacPolicy};
pub use refresh::RefreshService;
