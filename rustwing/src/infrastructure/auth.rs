use crate::error::CoreError;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,  // The user ID
    pub exp: usize, // Expiration time
}

pub struct AuthEngine;

impl AuthEngine {
    /// Hashes a password using Argon2id (industry standard)
    pub fn hash_password(password: &str) -> Result<String, CoreError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| CoreError::Internal(format!("Hashing error: {}", e)))?
            .to_string();
        Ok(password_hash)
    }

    /// Verifies a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Creates a JWT token valid for 24 hours
    pub fn create_jwt(user_id: Uuid, secret: &str) -> Result<String, CoreError> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: user_id,
            exp: expiration,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|_| CoreError::NotFound) // Replace with better CoreError variant later
    }

    /// Verifies a JWT and returns the User ID
    pub fn verify_jwt(token: &str, secret: &str) -> Result<Uuid, CoreError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| CoreError::NotFound)?; // Returns 401 equivalent

        Ok(token_data.claims.sub)
    }
}
