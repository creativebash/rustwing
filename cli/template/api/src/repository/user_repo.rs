use rustwing::prelude::*;
use crate::domain::user::User;

impl ModelName for User {
    fn table_name() -> &'static str {
        "users"
    }
}
