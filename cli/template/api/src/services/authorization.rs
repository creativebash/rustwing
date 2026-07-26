use rustwing::prelude::*;
use sqlx::PgPool;
use uuid::Uuid;

/// Require an active organization membership before accessing tenant data.
///
/// The route's tenant identifier is untrusted input; membership is checked
/// against the authenticated actor in the service layer.
pub async fn require_membership(
    db: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<(), CoreError> {
    let member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM organization_members
            WHERE organization_id = $1 AND user_id = $2 AND status = 'active'
        )",
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;

    if member {
        Ok(())
    } else {
        Err(CoreError::Forbidden)
    }
}
