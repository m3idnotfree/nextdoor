use serde::de::DeserializeOwned;

use crate::{
    error::ExtractError,
    request::{CloseFrame, Request},
    response::IntoResponse,
};

pub trait FromMesasge<S>: Sized {
    type Rejection: IntoResponse + Send;
    fn call(args: &Request, state: S) -> Result<Self, Self::Rejection>;
}

#[doc = "Extract of NextDoor"]
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> Json<T>
where
    T: Serialize,
{
    pub fn new(msg: T) -> Self {
        Json(msg)
    }
}

impl<T, S> FromMesasge<S> for Json<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;
    fn call(args: &Request, _: S) -> Result<Self, Self::Rejection> {
        let result = args
            .try_to_string()
            .map_err(ExtractError::FromStringError)?;

        let result: T = serde_json::from_str(&result).map_err(ExtractError::JsonError)?;

        Ok(Self(result))
    }
}

#[doc = "Extract of NextDoor"]
#[derive(Debug)]
pub struct State<S>(pub S);

impl<S> FromMesasge<S> for State<S>
where
    S: Clone + Send + Sync + 'static,
{
    type Rejection = ExtractError;
    fn call(_: &Request, state: S) -> Result<Self, Self::Rejection> {
        Ok(Self(state))
    }
}

/// wrapped in Arc<S>
#[doc = "Extract of NextDoor"]
#[derive(Debug)]
pub struct Close(pub Option<CloseFrame>);

impl<S> FromMesasge<S> for Close {
    type Rejection = ExtractError;
    fn call(args: &Request, _: S) -> Result<Self, Self::Rejection> {
        if args.is_empty() {
            Ok(Self(None))
        } else {
            Ok(Self(Some(
                serde_json::from_str(&args.try_to_string().unwrap()).unwrap(),
            )))
        }
    }
}

impl<S> FromMesasge<S> for String {
    type Rejection = ExtractError;
    fn call(args: &Request, _: S) -> Result<Self, Self::Rejection> {
        args.try_to_string().map_err(ExtractError::FromStringError)
    }
}

macro_rules! impl_from_message {
    ($($ty:ident),*) => {
        #[doc = "Extract of NextDoor"]
        #[derive(Debug)]
    $(  pub struct $ty(pub Vec<u8>);
        impl<S> FromMesasge<S> for $ty {
            type Rejection = ExtractError;
            fn call(args: &Request, _: S) -> Result<Self, Self::Rejection> {
                Ok(Self(args.to_vec()))
            }
        }
    )*
    };
}

impl_from_message!(Binary, Ping, Pong);

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use serde::{Deserialize, Serialize};
    use tokio_tungstenite::tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    };

    use crate::{
        error::ExtractError,
        extract::{Binary, Close, FromMesasge, Json, State},
        request::{Frames, Request},
    };

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        age: i32,
    }

    #[test]
    fn test_json_extractor() {
        let json_data = r#"{"name":"Alice","age":30}"#;
        let request = Request::new(Frames::Text, Bytes::from(json_data));

        let result: Result<Json<TestStruct>, ExtractError> = Json::call(&request, ());
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(
            json.0,
            TestStruct {
                name: "Alice".to_string(),
                age: 30
            }
        );
    }

    #[test]
    fn test_json_extractor_invalid_json() {
        let invalid_json = r#"{"name":"Alice","age":invalid}"#;
        let request = Request::new(Frames::Text, Bytes::from(invalid_json));

        let result = Json::<TestStruct>::call(&request, ());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExtractError::JsonError(_)));
    }

    #[test]
    fn test_state_extractor() {
        let json_data = r#"{"name":"Alice","age":30}"#;
        let request = Request::new(Frames::Text, Bytes::from(json_data));
        let state = "test_state".to_string();

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
}
