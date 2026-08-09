use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{
    error::{ErrorBody, ErrorResponse},
    http::{
        dtos::user_dto::{AuthResponse, LoginRequest, RegisterRequest, UpdateUser, UserResponse},
        handlers::{self, auth_routes, root, user_routes},
    },
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "{{project_name}} API",
        version = "0.1.0",
        description = "Generated Rustwing API"
    ),
    servers(
        (url = "http://localhost:3000", description = "Local development server")
    ),
    paths(
        root::live,
        root::ready,
        auth_routes::register,
        auth_routes::login,
        user_routes::get_current_user,
        user_routes::update_current_user,
        user_routes::delete_current_user,
        // rustwing:openapi-paths
    ),
    components(
        schemas(
            ErrorBody,
            ErrorResponse,
            root::HealthResponse,
            RegisterRequest,
            LoginRequest,
            UpdateUser,
            AuthResponse,
            UserResponse,
            // rustwing:openapi-schemas
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Users", description = "User account endpoints"),
        // rustwing:openapi-tags
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
