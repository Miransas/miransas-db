use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;

const TOKEN_EXPIRY_HOURS: i64 = 24;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_token(secret: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let exp = now
        .checked_add_signed(chrono::Duration::hours(TOKEN_EXPIRY_HOURS))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: "admin".to_string(),
        exp,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::BadRequest(format!("token creation failed: {e}")))
}

pub fn verify_token(secret: &str, token: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}
