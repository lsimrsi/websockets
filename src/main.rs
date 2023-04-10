use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State, TypedHeader,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use client::Client;
use tokio::sync::{broadcast, RwLock};

use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr, path::PathBuf};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::extract::connect_info::ConnectInfo;
use serde::{Deserialize, Serialize};

mod client;

type SharedState = Arc<ServerState>;

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    name: String,
    message: String,
}

pub struct Room {
    state: RwLock<RoomState>,
    tx: broadcast::Sender<ServerMessage>,
}

impl Room {
    pub fn new() -> Room {
        let (tx, _rx) = broadcast::channel(100);
        Room {
            state: RwLock::new(RoomState::new()),
            tx,
        }
    }
}

pub struct RoomState {
    users: Vec<String>,
    messages: Vec<ChatMessage>,
}

impl RoomState {
    pub fn new() -> RoomState {
        RoomState {
            users: Vec::new(),
            messages: Vec::new(),
        }
    }
}

pub struct ServerState {
    clients: HashMap<SocketAddr, String>,
    room: Room,
}

#[derive(Serialize, Debug, Clone)]
pub enum ServerMessageType {
    AllMessages,
    NewMessage,
    NameTaken,
    NameRegistered,
    Joined,
}

#[derive(Deserialize, Debug)]
pub enum ClientMessageType {
    RegisterName,
    Chat,
}

/// Messages sent from server to client.
#[derive(Serialize, Debug, Clone)]
pub struct ServerMessage {
    msg_type: ServerMessageType,
    data: serde_json::Value,
}

/// Messages sent from client to server.
#[derive(Deserialize, Debug)]
pub struct ClientMessage {
    msg_type: ClientMessageType,
    data: serde_json::Value,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let shared_state = Arc::new(ServerState {
        clients: HashMap::new(),
        room: Room::new(),
    });

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", get(ws_handler))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(Arc::clone(&shared_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start of websocket negotiation).
/// After this completes, the actual switching from HTTP to websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers, such as user-agent of the browser.
async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    // Finalize the upgrade process by returning upgrade callback.
    // We can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

/// Actual websocket statemachine (one will be spawned per connection).
async fn handle_socket(socket: WebSocket, who: SocketAddr, state: SharedState) {
    let client = Client::new(socket, who, state);
    client.listen().await;
}
