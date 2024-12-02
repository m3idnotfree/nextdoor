//! # NextDoor
//! ```toml
//! nextdoor = "0.0.*"
//! serde = { version = "1.0.215", features = ["derive"] }
//! tokio = { version = "1.41.1", features = ["full"] }
//! ```
//!
//! ```ignore
//! use nextdoor::{
//!     extract::{Binary, Close, Json, Ping, Pong, State},
//!     request::{Frames, Request},
//!     response::{IntoResponse, Response, Status},
//!     NextDoor,
//! };
//! use serde::{Deserialize, Serialize};
//! // with state
//! // use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut router = NextDoor::new();
//!     // with_state
//!     // let app_state = AppState {
//!     //     id: "test id".to_string(),
//!     //     secret: "test secret".to_string(),
//!     // };
//!     // let mut router = NextDoor::with_state(Arc::new(app_state));
//!
//!     router
//!         .text(param_empty_return_unit)
//!         .text(param_string_return_unit)
//!         .text(param_json_return_unit)
//!         .text(param_string_return_str)
//!         .text(param_string_return_string)
//!         .text(param_string_return_option_some)
//!         .text(param_string_return_option_none)
//!         .text(param_string_return_ok)
//!         .text(param_string_return_err)
//!         .text(param_json_return_json)
//!         // .text(param_app_store_return_unit)
//!         // .text(param_json_app_store_return_unit);
//!         .binary(binary)
//!         .ping(ping)
//!         .pong(pong)
//!         .close(close);
//!
//!     let req = Request::new(Frames::Text, Bytes::from("test text"));
//!     let response = router.handler(req).await;
//!
//!     // Features = "client"
//!     // nextdoor::connect(router, "url").run().await.unwrap();
//! }
//!
//!
//! #[derive(Deserialize, Serialize)]
//! struct User {
//!     name: String,
//!     id: String,
//! }
//!
//! struct AppState {
//!     id: String,
//!     secret: String,
//! }
//!
//! async fn param_empty_return_unit() {}
//! async fn param_string_return_unit(param: String) {}
//! async fn param_json_return_unit(Json(param): Json<User>) {}
//!
//! async fn param_string_return_str(param: String) -> String {
//!     "send to server".to_string()
//! }
//!
//! async fn param_string_return_string(param: String) -> &'static str {
//!     "send to server"
//! }
//!
//! async fn param_string_return_option_some(param: String) -> Option<String> {
//!     Some("send to server".to_string())
//! }
//!
//! async fn param_string_return_option_none(param: String) -> Option<String> {
//!     None
//! }
//!
//! async fn param_string_return_ok(param: String) -> Result<String, Error> {
//!     Ok("send to server".to_string())
//! }
//!
//! async fn param_string_return_err(param: String) -> Result<String, Error> {
//!     Err(Error::JsonError)
//! }
//!
//! async fn param_json_return_result(Json(param): Json<User>) -> Result<String, Error> {
//!     Ok("send to server".to_string())
//! }
//!
//! async fn param_json_return_json(Json(param): Json<User>) -> Result<Json<User>, Error> {
//!     let user = User {
//!         name: "test name".to_string(),
//!         id: "test id".to_string(),
//!     };
//!     Ok(Json::new(user))
//! }
//!
//! async fn param_app_store_return_unit(State(app_state): State<Arc<AppState>>) {}
//! async fn param_json_app_store_return_unit(
//!     Json(json): Json<User>,
//!     State(app_state): State<Arc<AppState>>,
//! ) {
//! }
//!
//! async fn binary(Binary(param): Binary) {}
//! async fn ping(Ping(param): Ping) {}
//! async fn pong(Pong(param): Pong) {}
//! async fn close(Close(param): Close) {}
//!
//! enum Error {
//!     JsonError,
//! }
//!
//! impl IntoResponse for Error {
//!     fn into_response(self) -> Response {
//!         match self {
//!             Self::JsonError => Response::error(Status::JsonError, "Failed parse Json"),
//!         }
//!     }
//! }
//! ```

pub mod error;
pub mod extract;
pub mod handler;
pub mod request;
pub mod response;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::*;

use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use handler::{ExtractorHandler, Handler, HandlerService};
use request::{Frames, Request};
use response::{Response, Status};
use tracing::{debug, instrument, warn};

pub struct EntryRoute<S> {
    handler: Box<dyn HandlerService<S> + Send + Sync>,
}

pub struct NextDoor<S = ()> {
    route: HashMap<Frames, Vec<EntryRoute<S>>>,
    state: S,
}

impl Default for NextDoor<Arc<()>> {
    fn default() -> Self {
        Self::new()
    }
}

impl NextDoor<Arc<()>> {
    pub fn new() -> Self {
        Self {
            route: HashMap::new(),
            state: Arc::new(()),
        }
    }
}

macro_rules! impl_router_route {
    ($method:ident,$frame:ident) => {
        impl<S> NextDoor<S>
        where
            S: Clone + Send + Sync + 'static,
        {
            pub fn $method<P, F>(&mut self, handler: F) -> &mut Self
            where
                F: Handler<P, S> + Clone + Send + Sync + 'static,
                P: Send + Sync + 'static,
            {
                self.route(Frames::$frame, handler)
            }
        }
    };
}

impl_router_route!(text, Text);
impl_router_route!(binary, Binary);
impl_router_route!(close, Close);
impl_router_route!(ping, Ping);
impl_router_route!(pong, Pong);

impl<S> NextDoor<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn with_state(state: S) -> NextDoor<S> {
        NextDoor {
            route: HashMap::new(),
            state,
        }
    }

    fn route<P, F>(&mut self, frame: Frames, handler: F) -> &mut Self
    where
        F: Handler<P, S> + Clone + Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        self.route.entry(frame).or_default().push(EntryRoute {
            handler: Box::new(ExtractorHandler {
                handler,
                _marker: PhantomData,
            }),
        });
        self
    }

    #[instrument(skip(self, req), fields(path = ?req.path))]
    pub async fn handler(&self, req: Request) -> Response {
        let routes = match self.route.get(&req.path) {
            Some(route) => {
                debug!("Found handler for frame type");
                route
            }
            None => {
                warn!("No handler found for frame type");
                return Response {
                    status: Status::NotFountPath,
                    body: String::from_utf8(req.to_vec()).unwrap(),
                };
            }
        };
        let mut last = Response {
            status: Status::NotFound,
            body: "".to_string(),
        };
        for route in routes.iter() {
            let result = route.handler.call(req.clone(), self.state.clone()).await;
            if result.status == Status::OK {
                return result;
            }
            last = result;
        }

        Response {
            status: Status::NotFound,
            body: last.body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use extract::State;
    use tokio_tungstenite::tungstenite::Message;

    #[tokio::test]
    async fn test_text_handler() {
        let mut router = NextDoor::new();
        router.text(|req: String| async move { req });

        let test_message = "Hello, World!";
        let request = Request::from_ws_message(Message::Text(test_message.to_string()));

        let response = router.handler(request).await;
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, test_message);
    }

    #[tokio::test]
    async fn test_not_found_handler() {
        let router = NextDoor::new();
        let test_message = "Hello, World!";
        let request = Request::from_ws_message(Message::Text(test_message.to_string()));

        let response = router.handler(request).await;
        assert_eq!(response.status, Status::NotFountPath);
    }

    #[tokio::test]
    async fn test_with_state() {
        let state = Arc::new("TestState".to_string());
        let mut router = NextDoor::with_state(state);
        router.text(|req: String, State(state): State<Arc<String>>| async move {
            format!("{} - {}", state, req)
        });

        let test_message = "Hello";
        let request = Request::from_ws_message(Message::Text(test_message.to_string()));

        let response = router.handler(request).await;
        assert_eq!(response.status, Status::OK);
        assert_eq!(response.body, format!("TestState - {}", test_message));
    }
}
