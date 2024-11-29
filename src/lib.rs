pub mod error;
pub mod extract;
pub mod handler;
pub mod request;
pub mod response;

use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use handler::{ExtractorHandler, Handler, HandlerService};
use request::{Frames, Request};
use response::{Response, Status};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, instrument, warn};

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

#[cfg(feature = "client")]
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Failed to websocket: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),
}

#[cfg(feature = "client")]
#[instrument(skip(router))]
pub async fn connect<S>(router: NextDoor<S>, url: &str) -> Result<(), ConnectError>
where
    S: Clone + Send + Sync + 'static,
{
    use futures_util::{SinkExt, StreamExt};

    info!("Establishing WebSocket connection");
    let (ws_stream, response) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();
    info!(status = ?response.status(), "WebSocket connection established");

    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                debug!(?msg, "Received WebSocket message");
                let request = Request::from_ws_message(msg);
                let response = router.handler(request).await;
                debug!(status = ?response.status, "Sending successful response");
                if response.status.is_success() {
                    write.send(Message::Text(response.body)).await?;
                } else {
                    warn!(
                        status = ?response.status,
                        body = %response.body,
                        "Handler returned error response"
                    );
                }
            }
            Err(e) => {
                error!(error = %e, "Error receiving WebSocket message");
                break;
            }
        }
    }

    Ok(())
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
