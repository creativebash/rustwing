use rustwing::repository::traits::ModelName;
use crate::domain::user::User;

impl ModelName for User {
    fn table_name() -> &'static str {
        "users"
    }
}
