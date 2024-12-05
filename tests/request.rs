use bytes::Bytes;
use nextdoor::request::{Frames, Request};
use tokio_tungstenite::tungstenite::{
    protocol::{
        frame::{coding::CloseCode, Frame, FrameHeader},
        CloseFrame,
    },
    Message,
};

#[test]
fn test_request_new() {
    let message = Message::Text("hello".to_string());
    let body = Bytes::from("hello");
    let request = Request::new(Frames::Text, body.clone());

    assert_eq!(request.path, Frames::Text);
    assert_eq!(request.body(), body);

    assert_eq!(request.into_ws_message(), message);
}

#[test]
fn test_from_ws_message_text() {
    let message = Message::Text("hello".to_string());
    let request = Request::from_ws_message(message.clone());

    assert_eq!(request.path, Frames::Text);
    assert_eq!(request.body(), Bytes::from("hello"));
    assert_eq!(request.into_ws_message(), message);
}

#[test]
fn test_from_ws_message_binary() {
    let data = vec![1, 2, 3, 4];
    let message = Message::Binary(data.clone());
    let request = Request::from_ws_message(message.clone());

    assert_eq!(request.path, Frames::Binary);
    assert_eq!(request.body(), Bytes::from(data));
    assert_eq!(request.into_ws_message(), message);
}

#[test]
fn test_from_ws_message_ping() {
    let data = vec![1, 2, 3];
    let message = Message::Ping(data.clone());
    let request = Request::from_ws_message(message.clone());

    assert_eq!(request.path, Frames::Ping);
    assert_eq!(request.body(), Bytes::from(data));
    assert_eq!(request.into_ws_message(), message);
}

#[test]
fn test_from_ws_message_pong() {
    let data = vec![1, 2, 3];
    let message = Message::Pong(data.clone());
    let request = Request::from_ws_message(message.clone());

    assert_eq!(request.path, Frames::Pong);
    assert_eq!(request.body(), Bytes::from(data));
    assert_eq!(request.into_ws_message(), message);
}

#[test]
fn test_from_ws_message_close() {
    let close_frame = CloseFrame {
        code: CloseCode::Normal,
        reason: "closing".into(),
    };
    let message = Message::Close(Some(close_frame));
    let request = Request::from_ws_message(message.clone());

    assert_eq!(request.path, Frames::Close);
    assert_eq!(
        request.body(),
        Bytes::from(b"{\"reason\":\"closing\",\"code\":1000}".to_vec())
    );
    assert_eq!(request.into_ws_message(), message);

    let message = Message::Close(None);
    let request = Request::from_ws_message(message.clone());

    assert_eq!(request.path, Frames::Close);
    assert_eq!(request.body(), Bytes::from(b"".to_vec()));
    assert_eq!(request.into_ws_message(), message);
}

#[test]
fn test_from_ws_message_frame() {
    let message = Message::Frame(Frame::from_payload(
        FrameHeader::default(),
        b"test".to_vec(),
    ));
    let request = Request::from_ws_message(message);
    assert_eq!(request.path, Frames::Binary);
    assert_eq!(request.body(), Bytes::from("test"));

    let binary_data = vec![1, 2, 3, 4];
    let message = Message::Frame(Frame::ping(binary_data.clone()));
    let request = Request::from_ws_message(message);
    assert_eq!(request.path, Frames::Binary);
    assert_eq!(request.body(), Bytes::from(binary_data));
}

#[test]
fn test_into_ws_message() {
    let original_message = Message::Text("hello".to_string());
    let request = Request::new(Frames::Text, Bytes::from("hello"));
    let message = request.into_ws_message();
    assert_eq!(message, original_message);
}

#[test]
fn test_try_to_string() {
    let message = Message::Text("hello".to_string());
    let request = Request::from_ws_message(message);
    let result = request.try_to_string().unwrap();
    assert_eq!(result, "hello");

    let invalid_utf8 = vec![0xFF, 0xFF];
    let message = Message::Binary(invalid_utf8);
    let request = Request::from_ws_message(message);
    assert!(request.try_to_string().is_err());
}

#[test]
fn test_to_vec() {
    let data = vec![1, 2, 3, 4];
    let message = Message::Binary(data.clone());
    let request = Request::from_ws_message(message);
    assert_eq!(request.to_vec(), data);
}

#[test]
fn test_frames_equality() {
    assert_eq!(Frames::Text, Frames::Text);
    assert_ne!(Frames::Text, Frames::Binary);
    assert_eq!(Frames::Binary, Frames::Binary);
    assert_eq!(Frames::Close, Frames::Close);
    assert_eq!(Frames::Ping, Frames::Ping);
    assert_eq!(Frames::Pong, Frames::Pong);
}
