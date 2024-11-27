use crate::extract::Json;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    OK,
    NoContent,

    NotFound,

    NotImplemented,

    JsonError,
    NotFountPath,
    FromStringError,
}

impl Status {
    pub fn is_success(&self) -> bool {
        *self == Status::OK
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
            None => Response::new(Status::NotFound, "Not Found"),
        }
    }
}

impl<T> IntoResponse for Json<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_string(&self.0) {
            Ok(json) => Response::ok(json),
            Err(err) => Response::error(Status::JsonError, err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use crate::{
        extract::Json,
        response::{IntoResponse, Response, Status},
    };

    #[test]
    fn test_status_methods() {
        assert!(Status::OK.is_success());
        assert!(Status::NoContent.is_error());
        assert!(!Status::OK.is_error());

        assert!(Status::NotFound.is_error());
        assert!(Status::NotImplemented.is_error());
        assert!(!Status::NotFound.is_success());
    }

    #[test]
    fn test_response_creation() {
        let response = Response::new(Status::OK, "test");
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "test");

        let response = Response::ok("success");
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "success");

        let response = Response::error(Status::NotFound, "not found");
        assert_eq!(response.status, Status::NotFound);
        assert_eq!(response.body, "not found");
    }

    #[test]
    fn test_unit_into_response() {
        let response = ().into_response();
        assert_eq!(response.status, Status::NoContent);
        assert_eq!(response.body, "");
    }

    #[test]
    fn test_status_into_response() {
        let response = Status::OK.into_response();
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "");
    }

    #[test]
    fn test_string_into_response() {
        let response = "test".into_response();
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "test");

        let response = String::from("test").into_response();
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "test");
    }

    #[test]
    fn test_result_into_response() {
        let ok_result: Result<&str, Status> = Ok("success");
        let response = ok_result.into_response();
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "success");

        let err_result: Result<&str, Status> = Err(Status::NotFound);
        let response = err_result.into_response();
        assert_eq!(response.status, Status::NotFound);
        assert_eq!(response.body, "");
    }

    #[test]
    fn test_option_into_response() {
        let some_value: Option<&str> = Some("found");
        let response = some_value.into_response();
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "found");

        let none_value: Option<&str> = None;
        let response = none_value.into_response();
        assert_eq!(response.status, Status::NotFound);
        assert_eq!(response.body, "Not Found");
    }

    #[test]
    fn test_json_into_response() {
        #[derive(Serialize)]
        struct TestStruct {
            field: String,
        }

        let json = Json(TestStruct {
            field: "test".to_string(),
        });
        let response = json.into_response();
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, r#"{"field":"test"}"#);
    }

    #[test]
    fn test_response_string_conversion() {
        let response = Response::new(Status::OK, String::from("test"));
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "test");

        let response = Response::new(Status::OK, "test".to_string());
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, "test");
    }
}
