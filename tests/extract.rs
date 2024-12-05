use std::sync::Arc;

use bytes::Bytes;
use nextdoor::{
    error::ExtractError,
    extract::{Binary, Close, FromMesasge, Json, State},
    request::{Frames, Request},
};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message,
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestStruct {
    id: String,
    secret: i32,
}

#[derive(Debug, PartialEq, Eq)]
struct AppState {
    next: bool,
}

#[test]
fn test_json_extractor() {
    let json_data = r#"{"id":"Alice","secret":30}"#;
    let request = Request::new(Frames::Text, Bytes::from(json_data));

    let result = Json::<TestStruct>::call(&request, ());
    assert!(result.is_ok());

    let json = result.unwrap();
    assert_eq!(
        json.0,
        TestStruct {
            id: "Alice".to_string(),
            secret: 30
        }
    );
}

#[test]
fn test_json_extractor_invalid_json() {
    let invalid_json = r#"{"id":"Alice","secret":invalid}"#;
    let request = Request::new(Frames::Text, Bytes::from(invalid_json));

    let result = Json::<TestStruct>::call(&request, ());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ExtractError::JsonError(_)));
}

#[test]
fn test_state_extractor() {
    let json_data = r#"{"id":"Alice","secret":30}"#;
    let request = Request::new(Frames::Text, Bytes::from(json_data));
    let state = "test_state".to_string();

    let result = State::call(&request, state.clone());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, state);

    let state = Arc::new(AppState { next: true });

    let result = State::call(&request, state.clone());

    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, state);
}

#[test]
fn test_binary_extractor() {
    let data = vec![1, 2, 3, 4];
    let request = Request::new(Frames::Binary, Bytes::from(data.clone()));

    let result = Binary::call(&request, ());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, data);
}

#[test]
fn test_close_extractor() {
    let data = Message::Close(Some(CloseFrame {
        reason: "test reason".into(),
        code: CloseCode::Normal,
    }));

    let request = Request::from_ws_message(data);

    let result = Close::call(&request, ());
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.0.is_some());
    let result = result.0.unwrap();
    assert_eq!(result.reason, "test reason".to_string());
    assert_eq!(result.code, 1000);
}
