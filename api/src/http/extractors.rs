use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use rustwing::prelude::*;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::state::AppState;

pub struct AuthUser {
    #[allow(dead_code)]
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Missing or invalid Bearer token" })),
            ));
        }

        let token = &auth_header["Bearer ".len()..];

        match AuthEngine::verify_jwt(token, &state.jwt_secret) {
            Ok(user_id) => Ok(AuthUser { id: user_id }),
            Err(_) => Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid or expired token" })),
            )),
        }
    }
}
