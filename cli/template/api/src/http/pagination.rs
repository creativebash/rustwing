use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct CursorPagination {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}
