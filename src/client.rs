use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, instrument, warn};

use crate::{request::Request, NextDoor};

pub fn connect<S>(router: NextDoor<S>, url: &str) -> Client<S>
where
    S: Clone + Send + Sync + 'static,
{
    Client {
        url,
        router: Arc::new(router),
        capacity: 100,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Failed to websocket: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),
}

pub struct Client<'a, S> {
    url: &'a str,
    router: Arc<NextDoor<S>>,
    capacity: usize,
}

impl<'a, S> Client<'a, S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(router: NextDoor<S>, url: &'a str, capacity: usize) -> Self {
        Self {
            router: Arc::new(router),
            url,
            capacity,
        }
    }

    #[instrument(skip(self))]
    pub async fn run(self) -> Result<(), ConnectError> {
        info!("Establishing WebSocket connection");
        let (ws_stream, response) = connect_async(self.url).await?;
        let (mut write, mut read) = ws_stream.split();
        info!(status = ?response.status(), "WebSocket connection established");
        let (tx, mut rx) = mpsc::channel(self.capacity);

        let recv_task = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(msg) => {
                        debug!(?msg, "Received WebSocket message");
                        let request = Request::from_ws_message(msg);
                        let response = self.router.handler(request).await;
                        debug!(status = ?response.status, "Sending successful response");
                        if response.status.is_success() {
                            if tx.send(Message::Text(response.body)).await.is_err() {
                                break;
                            }
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
        });

        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    error!(error = %e, "Error sending WebSocket message");
                    break;
                }
            }
        });

        let shutdown = async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!(error = %e, "Failed to listen for ctrl-c signal");
            }
            info!("Received shutdown signal");
        };

        tokio::select! {
            _ = recv_task => {}
            _ = send_task => {}
            _ = shutdown => {
                info!("Shutting down gracefully");
            }
        }

        Ok(())
    }

    pub fn set_capacity(&mut self, capacity: usize) -> &mut Self {
        self.capacity = capacity;

        self
    }
}
