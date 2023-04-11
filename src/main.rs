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
use serde_json::json;
use tokio::sync::{mpsc::Sender, RwLock};

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

type SharedState = Arc<Server>;

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    name: String,
    message: String,
}

pub struct Server {
    // rooms: HashMap<u32, broadcast::Sender<ServerMessage>>,
    /// Put entire server state behind one RwLock to prevent deadlocks.
    state: RwLock<ServerState>,
}

impl Server {
    pub fn new() -> Server {
        let room_messages = [(1, Vec::new())].into();

        let state = ServerState {
            sockets: HashMap::new(),
            room_messages,
        };
        Server {
            state: RwLock::new(state),
        }
    }

    pub async fn get_messages_in_room(&self, room_number: u32) -> Vec<ChatMessage> {
        match self.state.read().await.room_messages.get(&room_number) {
            Some(m) => m.clone(),
            None => Vec::new(),
        }
    }

    pub async fn add_socket(&self, socket_addr: SocketAddr, sender: Sender<ServerMessage>) {
        self.state.write().await.sockets.insert(
            socket_addr,
            SocketState {
                name: "".to_string(),
                sender,
                room: 1,
            },
        );
    }

    pub async fn join_room(&self, desired_room: u32, socket_addr: SocketAddr) {
        // First check if user is subscribing to a room they are already in.
        // let mut current_room = 0;
        let mut name = "".to_string();
        if let Some(socket) = self.state.write().await.sockets.get_mut(&socket_addr) {
            socket.room = desired_room;
            name = socket.name.clone();
        }

        //  Send message to all users in room letting them know a new user joined.
        let server_msg = ServerMessage {
            msg_type: ServerMessageType::Joined,
            data: json!(format!("{} joined.", name)),
        };
        let sockets = &self.state.read().await.sockets;

        for socket in sockets.values().filter(|s| s.room == desired_room) {
            let _ = socket.sender.send(server_msg.clone()).await;
        }
    }

    pub async fn is_name_available(&self, desired_name: &str) -> bool {
        let sockets = &self.state.read().await.sockets;
        let names: Vec<&String> = sockets.values().map(|s| &s.name).collect();

        !names.contains(&&desired_name.to_string())
    }

    pub async fn set_name(&self, name: String, socket_addr: SocketAddr) {
        if let Some(socket) = self.state.write().await.sockets.get_mut(&socket_addr) {
            socket.name = name;
        }
    }

    pub async fn send_message(&self, room: u32, data: serde_json::Value) {
        let sockets = &self.state.read().await.sockets;

        let server_msg = ServerMessage {
            msg_type: ServerMessageType::NewMessage,
            data,
        };

        for socket in sockets.values().filter(|s| s.room == room) {
            let _ = socket.sender.send(server_msg.clone()).await;
        }
    }
}

pub struct ServerState {
    sockets: HashMap<SocketAddr, SocketState>,
    room_messages: HashMap<u32, Vec<ChatMessage>>,
    // room_users: HashMap<u32, Vec<String>>,
}

/// Ties state for individual socket to a SocketAddr in server's state.
pub struct SocketState {
    /// User's name.
    name: String,
    sender: Sender<ServerMessage>,
    /// The room user is currently in.
    room: u32,
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
    let shared_state = Arc::new(Server::new());

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
