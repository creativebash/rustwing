use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[utoipa::path(
    get,
    path = "/health/live",
    tag = "Health",
    operation_id = "health",
    responses(
        (status = 200, description = "API is healthy", body = HealthResponse)
    )
)]
pub async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[utoipa::path(get, path = "/health/ready", tag = "Health", operation_id = "readiness", responses((status = 200, body = HealthResponse), (status = 503, body = HealthResponse)))]
pub async fn ready(
    State(state): State<crate::state::AppState>,
) -> (StatusCode, Json<HealthResponse>) {
    let ready = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db),
    )
    .await
    .is_ok_and(|result| result.is_ok());
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let value = if ready { "ready" } else { "not_ready" };
    (
        status,
        Json(HealthResponse {
            status: value.to_string(),
        }),
    )
}
