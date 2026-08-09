use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, Window>>>,
    limit: u64,
    duration: Duration,
}
struct Window {
    started: Instant,
    count: u64,
}

impl RateLimiter {
    pub fn new(limit: u64, duration: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            limit: limit.max(1),
            duration,
        }
    }
    fn allow(&self, key: &str) -> bool {
        let mut windows = self.inner.lock().expect("rate-limit mutex poisoned");
        let window = windows.entry(key.to_owned()).or_insert_with(|| Window {
            started: Instant::now(),
            count: 0,
        });
        if window.started.elapsed() >= self.duration {
            window.started = Instant::now();
            window.count = 0;
        }
        if window.count >= self.limit {
            false
        } else {
            window.count += 1;
            true
        }
    }
}

pub async fn enforce(State(limiter): State<RateLimiter>, request: Request, next: Next) -> Response {
    let key = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connection| connection.0.ip().to_string())
        .unwrap_or_else(|| "unknown".into());
    if limiter.allow(&key) {
        next.run(request).await
    } else {
        (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_after_process_local_limit() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.allow("client-a"));
        assert!(limiter.allow("client-a"));
        assert!(!limiter.allow("client-a"));
        assert!(limiter.allow("client-b"));
    }
}
