use serde::{de::DeserializeOwned, Serialize};

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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
        impl<S> FromMesasge<S> for $ty
        {
            type Rejection = ExtractError;
            fn call(args: &Request, _: S) -> Result<Self, Self::Rejection> {
                Ok(Self(args.to_vec()))
            }
        }

        impl Clone for $ty {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }
    )*
    };
}

impl_from_message!(Binary, Ping, Pong);
