use std::string::FromUtf8Error;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame as TCloseFrame},
    Message,
};

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
}

/// CloseFrame of Nextdoor
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloseFrame {
    pub reason: String,
    pub code: u16,
}

impl Request {
    pub fn new(path: Frames, body: Bytes) -> Self {
        Self { path, body }
    }

    pub fn from_ws_message(message: Message) -> Self {
        let (frame_type, body) = match message.clone() {
            Message::Text(text) => (Frames::Text, Bytes::from(text)),
            Message::Binary(data) => (Frames::Binary, Bytes::from(data)),
            Message::Ping(data) => (Frames::Ping, Bytes::from(data)),
            Message::Pong(data) => (Frames::Pong, Bytes::from(data)),
            Message::Close(frame) => {
                let data = frame
                    .map(|f| {
                        serde_json::to_string(&CloseFrame {
                            reason: f.reason.into_owned(),
                            code: f.code.into(),
                        })
                        .unwrap()
                    })
                    .unwrap_or_default();
                (Frames::Close, Bytes::from(data))
            }
            // Raw frame. Note, that you’re not going to get this value while reading the message.
            Message::Frame(frame) => (Frames::Binary, Bytes::from(frame.into_data())),
        };

        Self {
            path: frame_type,
            body,
        }
    }

    pub fn into_ws_message(self) -> Message {
        match self.path {
            Frames::Text => Message::Text(self.try_to_string().unwrap()),
            Frames::Binary => Message::Binary(self.body.to_vec()),
            Frames::Ping => Message::Ping(self.body.to_vec()),
            Frames::Pong => Message::Pong(self.body.to_vec()),
            Frames::Close => {
                if self.body.is_empty() {
                    Message::Close(None)
                } else {
                    let data: CloseFrame =
                        serde_json::from_str(&self.try_to_string().unwrap()).unwrap();

                    Message::Close(Some(TCloseFrame {
                        reason: data.reason.into(),
                        code: CloseCode::from(data.code),
                    }))
                }
            }
        }
    }

    pub fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.body.to_vec())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.body.to_vec()
    }

    pub fn len(&self) -> usize {
        self.body.len()
    }

    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }

    pub fn body(&self) -> Bytes {
        self.body.clone()
    }
}
