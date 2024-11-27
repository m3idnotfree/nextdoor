use std::string::FromUtf8Error;

use crate::response::{IntoResponse, Response, Status};

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Not found: {0}")]
    NotFound(String),
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(msg) => Response::error(Status::NotFound, msg),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("Failed to parse UTF-8 string: {0}")]
    FromStringError(#[from] FromUtf8Error),
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl IntoResponse for ExtractError {
    fn into_response(self) -> Response {
        match self {
            Self::FromStringError(e) => Response::error(
                Status::FromStringError,
                format!("Failed to parse request body as UTF-8: {}", e),
            ),
            Self::JsonError(e) => Response::error(
                Status::JsonError,
                format!("Failed to parse JSON payload: {}", e),
            ),
        }
    }
}
