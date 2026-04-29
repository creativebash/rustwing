use crate::{
    domain::user::User,
    error::AppError,
    http::dtos::user_dto::CreateUser,
    repository::user_repo::InsertUser,
};
use rustwing::{
    infrastructure::llm::{LlmRef, LlmRequest},
    repository::generic_crud,
};
use sqlx::PgPool;

pub struct UserService;

impl UserService {
    pub async fn create_user_with_bio(
        pool: &PgPool,
        llm: &LlmRef,
        mut payload: CreateUser,
    ) -> Result<User, AppError> {
        if payload.bio.is_none() {
            let prompt = format!(
                "Write a fun, 1-sentence fictional biography for a user named {}.",
                payload.username
            );

            match llm
                .complete(LlmRequest {
                    prompt,
                    max_tokens: Some(50),
                })
                .await
            {
                Ok(response) => payload.bio = Some(response.completion),
                Err(e) => tracing::warn!(
                    "Failed to generate AI bio for {}, falling back to None: {:?}",
                    payload.username,
                    e
                ),
            }
        }

        let insert = InsertUser::from(payload);
        Ok(generic_crud::insert::<User, InsertUser>(pool, &insert).await?)
    }
}
