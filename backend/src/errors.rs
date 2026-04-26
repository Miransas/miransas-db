use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

use crate::utils::crypto::CryptoError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authorization failed")]
    Unauthorized,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Database(_) | Self::Crypto(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::BadRequest(_) => "bad_request",
            Self::NotFound(_) => "not_found",
            Self::Database(_) => "database_error",
            Self::Crypto(_) => "secret_encryption_error",
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::Unauthorized => "missing or invalid bearer token".to_string(),
            Self::BadRequest(message) | Self::NotFound(message) => message.clone(),
            Self::Database(_) => "database operation failed".to_string(),
            Self::Crypto(_) => "secret encryption failed".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            Self::Database(e) => {
                error!(error = %e, error_detail = ?e, "request failed: database error");
            }
            Self::Crypto(e) => {
                error!(error = %e, "request failed: crypto error — MIRANSAS_ENCRYPTION_KEY may be missing or invalid");
            }
            _ => {}
        }

        let status = self.status_code();
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code(),
                message: self.public_message(),
            },
        };

        (status, Json(body)).into_response()
    }
}
