use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

use crate::{errors::AppError, state::AppState, utils::jwt};

pub async fn require_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Some(header_value) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return Err(AppError::Unauthorized);
    };

    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    jwt::verify_token(&state.config.jwt_secret, token)?;
    Ok(next.run(req).await)
}
