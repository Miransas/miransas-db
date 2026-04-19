use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{errors::AppError, state::AppState, utils::jwt};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    if !constant_time_eq(
        input.password.as_bytes(),
        state.config.admin_password.as_bytes(),
    ) {
        return Err(AppError::Unauthorized);
    }

    let token = jwt::create_token(&state.config.jwt_secret)?;
    Ok(Json(LoginResponse {
        token,
        expires_in: 24 * 3600,
    }))
}

/// Timing-safe byte slice comparison.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for i in 0..max_len {
        let l = left.get(i).copied().unwrap_or(0);
        let r = right.get(i).copied().unwrap_or(0);
        diff |= (l ^ r) as usize;
    }
    diff == 0
}
