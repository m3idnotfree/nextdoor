use serde::Serialize;

use nextdoor::{
    extract::Json,
    response::{IntoResponse, Response, Status},
};

macro_rules! into_response_success {
    (Result, $ok:ty, $success:expr, $status:ident) => {
        let result: Result<$ok, Status> = Ok($success);
        let response = result.into_response();
        assert_eq!(response.status, Status::$status);
        assert_eq!(response.body, "");
    };
    (Result, $ok:ty, $success:expr, $status:ident, $body:literal) => {
        let result: Result<$ok, Status> = Ok($success);
        let response = result.into_response();
        assert_eq!(response.status, Status::$status);
        assert_eq!(response.body, $body);
    };
    (Option, $ok:ty, $success:expr, $status:ident) => {
        let result: Result<$ok, Status> = Ok($success);
        let response = result.into_response();
        assert_eq!(response.status, Status::$status);
        assert_eq!(response.body, "");
    };
    (Option, $ok:ty, $success:expr, $status:ident, $body:literal) => {
        let result: Result<$ok, Status> = Ok($success);
        let response = result.into_response();
        assert_eq!(response.status, Status::$status);
        assert_eq!(response.body, $body);
    };
    ($t:expr, $status:ident, $body:literal) => {
        let response = $t.into_response();
        assert_eq!(response.status, Status::$status);
        assert_eq!(response.body, $body);
    };
    ($t:expr, $status:ident) => {
        let response = $t.into_response();
        assert_eq!(response.status, Status::$status);
        assert_eq!(response.body, "");
    };
}

#[test]
fn test_status_methods() {
    assert!(Status::OK.is_success());
    assert!(Status::NoContent.is_error());
    assert!(Status::NotFound.is_error());
    assert!(Status::Reconnect.is_reconnect());
    assert!(Status::NotImplemented.is_error());
    assert!(Status::JsonError.is_error());
    assert!(Status::NotFountPath.is_error());
    assert!(Status::FromStringError.is_error());
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
fn test_into_response() {
    into_response_success!((), NoContent);
    into_response_success!((Status::OK, "tuple".to_string()), OK, "tuple");
    into_response_success!(Status::OK, OK);
    into_response_success!("test".to_string(), OK, "test");
    into_response_success!("test", OK, "test");
}

#[test]
fn test_result_into_response() {
    into_response_success!(Result, &str, "success", OK, "success");
    into_response_success!(Result, (), (), NoContent);
    into_response_success!(
        Result,
        (Status, String),
        (Status::OK, "tuple test".to_string()),
        OK,
        "tuple test"
    );
    into_response_success!(Result, Status, Status::Reconnect, Reconnect);
    into_response_success!(Result, String, "string test".to_string(), OK, "string test");

    let err_result: Result<&str, Status> = Err(Status::NotFound);
    let response = err_result.into_response();
    assert_eq!(response.status, Status::NotFound);
    assert_eq!(response.body, "");
}

#[test]
fn test_option_into_response() {
    into_response_success!(Option, &str, "success", OK, "success");
    into_response_success!(Option, (), (), NoContent);
    into_response_success!(
        Option,
        (Status, String),
        (Status::OK, "tuple test".to_string()),
        OK,
        "tuple test"
    );
    into_response_success!(Option, Status, Status::Reconnect, Reconnect);
    into_response_success!(Option, String, "string test".to_string(), OK, "string test");

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

    let json = Json::new(TestStruct {
        field: "test".to_string(),
    });
    let response = json.into_response();
    assert_eq!(response.status, Status::OK);
    assert_eq!(response.body, r#"{"field":"test"}"#);
}
