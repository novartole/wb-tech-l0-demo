use Error::*;

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    #[error("connection to postgres db failed: {0}")]
    PgConnFailed(#[from] bb8::RunError<tokio_postgres::Error>),

    #[error("postgres failed to execute query: {0}")]
    PgQueryFailed(#[from] tokio_postgres::Error),

    #[error("connection to redis cache failed: {0}")]
    RedisConnFailed(#[from] bb8::RunError<redis::RedisError>),

    #[error("redis failed to execute query: {0}")]
    RedisQueryFailed(#[from] redis::RedisError),

    #[error("cannot find '{target}' by '{id_name}={id_val}'")]
    NotFound {
        id_name: String,
        id_val: String,
        target: String,
    },
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        error!("{}", self.to_string());

        let (status, message) = match self {
            NotFound { .. } => return StatusCode::NOT_FOUND.into_response(),
            JsonRejection(rejection) => (rejection.status(), rejection.body_text()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "an internal server error occurred".to_owned(),
            ),
        };

        (status, Json(ErrorResponse { message })).into_response()
    }
}

impl Error {
    pub fn not_found(id_name: &str, id_val: impl ToString, target: &str) -> Error {
        Error::NotFound {
            id_name: id_name.to_owned(),
            id_val: id_val.to_string(),
            target: target.to_owned(),
        }
    }
}
