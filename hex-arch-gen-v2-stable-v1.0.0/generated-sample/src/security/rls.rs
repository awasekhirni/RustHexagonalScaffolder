//! Row-Level Security — sets the PostgreSQL `app.tenant_id` GUC on every
//! connection so RLS policies can filter rows transparently.

use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Sets the tenant GUC on the given pool's connection.
///
/// Production code calls this from a connection callback; the stub logs
/// so behaviour is observable.
pub async fn set_tenant_guc(pool: &PgPool, tenant_id: Uuid) -> Result<(), sqlx::Error> {
    let _ = pool.acquire().await?;
    info!(tenant_id = %tenant_id, "set tenant GUC (stub)");
    Ok(())
}
