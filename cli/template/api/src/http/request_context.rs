use axum::{
    extract::{FromRequestParts, Request},
    http::{HeaderName, HeaderValue, request::Parts},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;
use uuid::Uuid;

use crate::state::AppState;

pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestContext {
    pub request_id: String,
}

impl FromRequestParts<AppState> for RequestContext {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Self>()
            .cloned()
            .unwrap_or_else(|| Self {
                request_id: Uuid::now_v7().to_string(),
            }))
    }
}

pub async fn attach(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid(value))
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::now_v7().to_string());
    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
    });
    let method = request.method().clone();
    let uri = request.uri().path().to_owned();
    let span = tracing::info_span!("http.request", request_id = %request_id, %method, %uri);
    let mut response = async move {
        let response = next.run(request).await;
        tracing::info!(status = response.status().as_u16(), "request completed");
        response
    }
    .instrument(span)
    .await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}

fn valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, middleware, routing::get};
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };
    use tower::ServiceExt;

    struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for CapturedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn validates_safe_request_ids() {
        assert!(valid("provider:request-123"));
        assert!(!valid("contains a space"));
        assert!(!valid(&"x".repeat(129)));
    }

    #[tokio::test]
    async fn response_and_structured_span_propagate_request_id() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = output.clone();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_writer(move || CapturedWriter(writer.clone()))
            .finish();
        let _subscriber = tracing::subscriber::set_default(subscriber);
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(attach));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-request-id", "incoming-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.headers().get("x-request-id").unwrap(),
            "incoming-123"
        );
        let logs = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("incoming-123"));
        assert!(logs.contains("request_id"));
    }
}
