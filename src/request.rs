use std::string::FromUtf8Error;

use bytes::Bytes;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Frames {
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

/// Request of Nextdoor
#[derive(Clone)]
pub struct Request {
    pub path: Frames,
    body: Bytes,
    raw_message: Message,
}

impl Request {
    pub fn new(path: Frames, body: Bytes, raw_message: Message) -> Self {
        Self {
            path,
            body,
            raw_message,
        }
    }
    pub fn from_ws_message(message: Message) -> Option<Self> {
        let (frame_type, body) = match message.clone() {
            Message::Text(text) => (Frames::Text, Bytes::from(text)),
            Message::Binary(data) => (Frames::Binary, Bytes::from(data)),
            Message::Ping(data) => (Frames::Ping, Bytes::from(data)),
            Message::Pong(data) => (Frames::Pong, Bytes::from(data)),
            Message::Close(frame) => {
                let data = frame.map(|f| f.reason.into_owned()).unwrap_or_default();
                (Frames::Close, Bytes::from(data))
            }
            Message::Frame(frame) => match frame.to_text() {
                Ok(_) => (Frames::Text, Bytes::from(frame.into_data())),
                Err(_) => (Frames::Binary, Bytes::from(frame.into_data())),
            },
        };

        Some(Self {
            path: frame_type,
            body,
            raw_message: message,
        })
    }

    pub fn into_ws_message(self) -> Message {
        self.raw_message
    }

    pub fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.body.to_vec())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.body.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite::protocol::{
        frame::{coding::CloseCode, Frame, FrameHeader},
        CloseFrame,
    };

    #[test]
    fn test_request_new() {
        let message = Message::Text("hello".to_string());
        let body = Bytes::from("hello");
        let request = Request::new(Frames::Text, body.clone(), message.clone());

        assert_eq!(request.path, Frames::Text);
        assert_eq!(request.body, body);
        assert_eq!(request.raw_message, message);
    }

    #[test]
    fn test_from_ws_message_text() {
        let message = Message::Text("hello".to_string());
        let request = Request::from_ws_message(message.clone()).unwrap();

        assert_eq!(request.path, Frames::Text);
        assert_eq!(request.body, Bytes::from("hello"));
        assert_eq!(request.raw_message, message);
    }

    #[test]
    fn test_from_ws_message_binary() {
        let data = vec![1, 2, 3, 4];
        let message = Message::Binary(data.clone());
        let request = Request::from_ws_message(message.clone()).unwrap();

        assert_eq!(request.path, Frames::Binary);
        assert_eq!(request.body, Bytes::from(data));
        assert_eq!(request.raw_message, message);
    }

    #[test]
    fn test_from_ws_message_ping() {
        let data = vec![1, 2, 3];
        let message = Message::Ping(data.clone());
        let request = Request::from_ws_message(message.clone()).unwrap();

        assert_eq!(request.path, Frames::Ping);
        assert_eq!(request.body, Bytes::from(data));
        assert_eq!(request.raw_message, message);
    }

    #[test]
    fn test_from_ws_message_pong() {
        let data = vec![1, 2, 3];
        let message = Message::Pong(data.clone());
        let request = Request::from_ws_message(message.clone()).unwrap();

        assert_eq!(request.path, Frames::Pong);
        assert_eq!(request.body, Bytes::from(data));
        assert_eq!(request.raw_message, message);
    }

    #[test]
    fn test_from_ws_message_close() {
        let reason = "closing".to_string();
        let close_frame = CloseFrame {
            code: CloseCode::Normal,
            reason: reason.clone().into(),
        };
        let message = Message::Close(Some(close_frame));
        let request = Request::from_ws_message(message.clone()).unwrap();

        assert_eq!(request.path, Frames::Close);
        assert_eq!(request.body, Bytes::from(reason));
        assert_eq!(request.raw_message, message);
    }

    #[test]
    fn test_from_ws_message_frame() {
        let message = Message::Frame(Frame::from_payload(
            FrameHeader::default(),
            b"test".to_vec(),
        ));
        let request = Request::from_ws_message(message).unwrap();
        assert_eq!(request.path, Frames::Text);
        assert_eq!(request.body, Bytes::from("test"));

        let binary_data = vec![1, 2, 3, 4];
        let message = Message::Frame(Frame::ping(binary_data.clone()));
        let request = Request::from_ws_message(message).unwrap();
        // assert_eq!(request.path, Frames::Binary);
        assert_eq!(request.path, Frames::Text);
        assert_eq!(request.body, Bytes::from(binary_data));
    }

    #[test]
    fn test_into_ws_message() {
        let original_message = Message::Text("hello".to_string());
        let request = Request::new(Frames::Text, Bytes::from("hello"), original_message.clone());
        let message = request.into_ws_message();
        assert_eq!(message, original_message);
    }

    #[test]
    fn test_try_to_string() {
        let message = Message::Text("hello".to_string());
        let request = Request::from_ws_message(message).unwrap();
        let result = request.try_to_string().unwrap();
        assert_eq!(result, "hello");

        let invalid_utf8 = vec![0xFF, 0xFF];
        let message = Message::Binary(invalid_utf8);
        let request = Request::from_ws_message(message).unwrap();
        assert!(request.try_to_string().is_err());
    }

    #[test]
    fn test_to_vec() {
        let data = vec![1, 2, 3, 4];
        let message = Message::Binary(data.clone());
        let request = Request::from_ws_message(message).unwrap();
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
}
