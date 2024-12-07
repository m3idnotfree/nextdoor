use serde::Serialize;

use crate::extract::Json;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    OK,
    NoContent,

    NotFound,

    Reconnect,

    NotImplemented,

    JsonError,
    NotFountPath,
    FromStringError,
}

impl Status {
    pub fn is_success(&self) -> bool {
        *self == Status::OK
    }

    pub fn is_reconnect(&self) -> bool {
        *self == Status::Reconnect
    }

    pub fn is_error(&self) -> bool {
        *self != Status::OK
    }
}

pub struct Response {
    pub status: Status,
    pub body: String,
}

impl Response {
    pub fn new<I: Into<String>>(status: Status, body: I) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    pub fn ok<I: Into<String>>(body: I) -> Self {
        Self::new(Status::OK, body)
    }

    pub fn error<I: Into<String>>(status: Status, message: I) -> Self {
        Self::new(status, message)
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::new(Status::NoContent, "")
    }
}

impl IntoResponse for (Status, String) {
    fn into_response(self) -> Response {
        Response::new(self.0, self.1)
    }
}

impl IntoResponse for Status {
    fn into_response(self) -> Response {
        Response::new(self, "")
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::ok(self)
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        Response::ok(self)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(value) => value.into_response(),
            Err(err) => err.into_response(),
        }
    }
}

impl<T> IntoResponse for Option<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Some(value) => value.into_response(),
            None => Response::new(Status::NotFound, ""),
        }
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_string(&self.0) {
            Ok(json) => Response::ok(json),
            Err(err) => Response::error(Status::JsonError, err.to_string()),
        }
    }
}
