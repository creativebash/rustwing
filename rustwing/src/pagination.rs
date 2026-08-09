use crate::error::CoreError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Serialize;
use uuid::Uuid;

const CURSOR_VERSION: &str = "v1";

#[derive(Debug, Clone, Serialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

impl<T> CursorPage<T> {
    pub fn new(items: Vec<T>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }
}

pub fn encode_cursor(id: Uuid) -> String {
    format!("{CURSOR_VERSION}.{}", URL_SAFE_NO_PAD.encode(id.as_bytes()))
}

pub fn decode_cursor(cursor: &str) -> Result<Uuid, CoreError> {
    let (version, encoded) = cursor
        .split_once('.')
        .ok_or_else(|| CoreError::InvalidInput("invalid cursor".into()))?;
    if version != CURSOR_VERSION {
        return Err(CoreError::InvalidInput("unsupported cursor version".into()));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| CoreError::InvalidInput("invalid cursor".into()))?;
    Uuid::from_slice(&bytes).map_err(|_| CoreError::InvalidInput("invalid cursor".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{ContextV7, Timestamp};

    #[test]
    fn cursor_round_trips_without_exposing_uuid_text() {
        let id = Uuid::now_v7();
        let cursor = encode_cursor(id);
        assert!(!cursor.contains(&id.to_string()));
        assert_eq!(decode_cursor(&cursor).unwrap(), id);
    }

    #[test]
    fn invalid_cursors_are_clean_input_errors() {
        assert!(matches!(
            decode_cursor("bad"),
            Err(CoreError::InvalidInput(_))
        ));
        assert!(matches!(
            decode_cursor("v2.AA"),
            Err(CoreError::InvalidInput(_))
        ));
    }

    #[test]
    fn uuid_v7_context_orders_same_millisecond_ids() {
        let context = ContextV7::new();
        let timestamp = || Timestamp::from_unix(&context, 1_700_000_000, 123_000_000);
        let ids: Vec<_> = (0..10_000).map(|_| Uuid::new_v7(timestamp())).collect();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn concurrent_uuid_v7_generation_is_unique() {
        let workers: Vec<_> = (0..8)
            .map(|_| std::thread::spawn(|| (0..2_000).map(|_| Uuid::now_v7()).collect::<Vec<_>>()))
            .collect();
        let ids: Vec<_> = workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect();
        let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len());
        assert!(ids.iter().all(|id| id.get_version_num() == 7));
    }
}
