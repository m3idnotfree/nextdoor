use std::{sync::Arc, time::Duration};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{net::TcpStream, sync::mpsc, time::sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, instrument, warn};

use crate::{request::Request, NextDoor};

pub fn connect<S, T: Into<String>>(router: NextDoor<S>, url: T) -> Client<S>
where
    S: Clone + Send + Sync + 'static,
{
    Client {
        url: url.into(),
        router: Arc::new(router),
        capacity: 100,
        reconnect_config: None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Failed to websocket: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Max reconnection attempts reached")]
    MaxRetriesExceeded,
}

#[derive(Clone)]
pub struct Client<S> {
    url: String,
    router: Arc<NextDoor<S>>,
    capacity: usize,
    reconnect_config: Option<ReconnectConfig>,
}

impl<S> Client<S>
where
    S: Clone + Send + Sync + 'static,
{
    #[instrument(skip(self), fields(url = %self.url))]
    pub async fn run(self) -> Result<(), ConnectError> {
        let mut current_url = self.url;
        let mut retry_count = 0;
        let mut delay = self
            .reconnect_config
            .as_ref()
            .map_or(0, |config| config.initial_delay);

        loop {
            debug!("Establishing WebSocket connection");
            match connect_async(&current_url).await {
                Ok((ws_stream, response)) => {
                    let (write, read) = ws_stream.split();
                    debug!(status = ?response.status(), "WebSocket connection established");
                    let (tx, rx) = mpsc::channel(self.capacity);

                    let router = self.router.clone();
                    let recv_task = tokio::spawn(receive_messages(read, router, tx));
                    let send_task = tokio::spawn(send_messages(write, rx));

                    tokio::select! {
                        result = recv_task => {
                            match result {
                                Ok((should_reconnect,maybe_new_url)) => {
                                    if should_reconnect {
                                        if let Some(new_url) = maybe_new_url {
                                            info!("Initiating reconnection to new URL: {}", new_url);
                                            current_url = new_url;
                                        } else {
                                            info!("Initiating reconnection to same URL");
                                        }

                                        sleep(Duration::from_secs(1)).await;
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "Receive task join error");
                                    println!( "Receive task join error");
                                }
                            }
                       }
                        _ = send_task => {}
                        _ = shutdown() => {
                            info!("Shutting down gracefully");
                            break;
                        }
                    }
                }
                Err(e) => {
                    let Some(reconnect_config) = &self.reconnect_config else {
                        return Err(ConnectError::WsError(e));
                    };

                    if retry_count >= reconnect_config.max_retries {
                        error!(
                            error = %e,
                            max_retries = reconnect_config.max_retries,
                            "Max reconnection attempts reached"
                        );
                        return Err(ConnectError::MaxRetriesExceeded);
                    }

                    retry_count += 1;
                    delay = calculate_next_delay(
                        delay,
                        reconnect_config.backoff_factor,
                        reconnect_config.max_delay,
                    );

                    warn!(
                        error = %e,
                        retry_count,
                        next_attempt_delay_ms = delay,
                        max_retries = reconnect_config.max_retries,
                        "Connection failed, attempting to reconnect"
                    );

                    sleep(Duration::from_millis(delay)).await;
                    continue;
                }
            }
        }

        Ok(())
    }

    pub fn set_capacity(&mut self, capacity: usize) -> &mut Self {
        self.capacity = capacity;

        self
    }

    pub fn with_reconnect_config(mut self, config: ReconnectConfig) -> Self {
        self.reconnect_config = Some(config);
        self
    }
}

#[derive(Clone)]
pub struct ReconnectConfig {
    pub initial_delay: u64,
    pub max_delay: u64,
    pub max_retries: u32,
    pub backoff_factor: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: 1000,
            max_delay: 30000,
            max_retries: 8,
            backoff_factor: 1.5,
        }
    }
}
fn calculate_next_delay(current_delay: u64, backoff_factor: f64, max_delay: u64) -> u64 {
    ((current_delay as f64 * backoff_factor) as u64).min(max_delay)
}

async fn handle_message<S>(
    msg: Message,
    router: Arc<NextDoor<S>>,
    tx: &mpsc::Sender<Message>,
) -> Option<(bool, Option<String>)>
where
    S: Clone + Send + Sync + 'static,
{
    debug!(?msg, "Received WebSocket message");
    let request = Request::from_ws_message(msg);
    let response = router.handler(request).await;
    debug!(status = ?response.status, "Sending successful response");

    if response.status.is_reconnect() {
        return Some((true, Some(response.body)));
    }

    if response.status.is_success() {
        if tx.send(Message::Text(response.body)).await.is_err() {
            return Some((false, None));
        }
    } else {
        warn!(
            status = ?response.status,
            body = %response.body,
            "Handler returned error response"
        );
    }
    None
}

async fn receive_messages<S>(
    mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    router: Arc<NextDoor<S>>,
    tx: mpsc::Sender<Message>,
) -> (bool, Option<String>)
where
    S: Clone + Send + Sync + 'static,
{
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let Some(result) = handle_message(msg, router.clone(), &tx).await {
                    return result;
                }
            }
            Err(e) => {
                error!(error = %e, "Error receiving WebSocket message");
                return (false, None);
            }
        }
    }
    (false, None)
}

async fn send_messages(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut rx: mpsc::Receiver<Message>,
) {
    while let Some(msg) = rx.recv().await {
        if let Err(e) = write.send(msg).await {
            error!(error = %e, "Error sending WebSocket message");
            break;
        }
    }
}

async fn shutdown() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(error = %e, "Failed to listen for ctrl-c signal");
    }
}
