use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, TypedHeader,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use serde_json::json;
use tokio::sync::{
    broadcast,
    mpsc::{self, Sender},
    RwLock,
};

use std::{net::SocketAddr, path::PathBuf};
use std::{ops::ControlFlow, sync::Arc};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::extract::connect_info::ConnectInfo;
use serde::{Deserialize, Serialize};

//allows to split the websocket stream into separate TX and RX branches
use futures::{stream::StreamExt, SinkExt};

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
    room: Room,
}

#[derive(Serialize, Debug, Clone)]
pub enum ServerMessageType {
    AllMessages,
    NewMessage,
    NameTaken,
    NameRegistered,
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
    let shared_state = Arc::new(ServerState { room: Room::new() });

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
async fn handle_socket(mut socket: WebSocket, who: SocketAddr, state: SharedState) {
    // Send a ping (unsupported by some browsers) to kick things off.
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("<<< Pinged {}...", who);
    } else {
        println!("Could not send ping to {}.", who);
        return;
    }

    let all_messages = ServerMessage {
        msg_type: ServerMessageType::AllMessages,
        data: json!(*state.room.state.read().await.messages),
    };

    let all_messages = match serde_json::to_string(&all_messages) {
        Ok(s) => s,
        Err(_) => {
            println!("Could not convert all messages to string");
            return;
        }
    };

    if !socket.send(Message::Text(all_messages)).await.is_ok() {
        println!("Could not send all messages to client.");
        return;
    };

    let mut rx = state.room.tx.subscribe();

    let (mut sink, mut stream) = socket.split();
    let (sender, mut receiver) = mpsc::channel::<ServerMessage>(16);

    // Forwards messages from mpsc to sink.
    let mut forward_task = tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            println!("<<< {}: {:?}", who, message);
            if sink
                .send(Message::Text(json!(message).to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Task to receive messages from the room.
    let rx_sender = sender.clone();
    let mut rx_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if rx_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Task to receive messages from the client.
    let mut stream_sender = sender.clone();
    let mut stream_task = tokio::spawn(async move {
        loop {
            if let Some(Ok(msg)) = stream.next().await {
                if process_message(msg, &mut stream_sender, who, state.clone())
                    .await
                    .is_break()
                {
                    println!("Break received for {}.", who);
                    break;
                }
            }
        }
    });

    // If any task runs to completion, abort the others.
    tokio::select! {
        _ = (&mut forward_task) => {rx_task.abort(); stream_task.abort();},
        _ = (&mut rx_task) => {forward_task.abort(); stream_task.abort();},
        _ = (&mut stream_task) => {forward_task.abort(); rx_task.abort();},
    };

    println!("Websocket context {} destroyed", who);
}

async fn process_message(
    msg: Message,
    sender: &mut Sender<ServerMessage>,
    who: SocketAddr,
    state: SharedState,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => match serde_json::from_str::<ClientMessage>(&t) {
            Ok(client_msg) => {
                return process_client_message(client_msg, sender, who, state).await;
            }
            Err(_) => println!(">>> {} sent str: {:?}", who, t),
        },
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {} somehow sent close message without CloseFrame", who);
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {} sent pong with {:?}", who, v);
        }
        // You should never need to manually handle Message::Ping.
        // Axum's websocket library will do so automagically.
        Message::Ping(v) => {
            println!(">>> {} sent ping with {:?}", who, v);
        }
    }
    ControlFlow::Continue(())
}

async fn process_client_message(
    client_msg: ClientMessage,
    sender: &mut Sender<ServerMessage>,
    who: SocketAddr,
    state: SharedState,
) -> ControlFlow<(), ()> {
    println!(">>> client message from {}: {:?}", who, client_msg);
    match client_msg.msg_type {
        ClientMessageType::RegisterName => {
            let name: String = match serde_json::from_value(client_msg.data) {
                Ok(n) => n,
                Err(err) => {
                    println!("Could not parse name: {}", err);
                    return ControlFlow::Continue(());
                }
            };

            if name.is_empty() {
                println!("Name was empty.");
                return ControlFlow::Continue(());
            }

            {
                let users = &state.room.state.read().await.users;

                if users.contains(&name) {
                    let server_msg = ServerMessage {
                        msg_type: ServerMessageType::NameTaken,
                        data: json!(""),
                    };
                    if sender.send(server_msg).await.is_err() {
                        return ControlFlow::Break(());
                    };
                    return ControlFlow::Continue(());
                }
            }

            {
                let users = &mut state.room.state.write().await.users;
                users.push(name);
            }

            let server_msg = ServerMessage {
                msg_type: ServerMessageType::NameRegistered,
                data: json!(""),
            };
            if sender.send(server_msg).await.is_err() {
                return ControlFlow::Break(());
            };
        }
        ClientMessageType::Chat => {
            let chat_msg: ChatMessage = match serde_json::from_value(client_msg.data.clone()) {
                Ok(cm) => cm,
                Err(err) => {
                    println!("Could not deserialize chat message: {}", err);
                    return ControlFlow::Continue(());
                }
            };
            let server_msg = ServerMessage {
                msg_type: ServerMessageType::NewMessage,
                data: client_msg.data,
            };
            state.room.state.write().await.messages.push(chat_msg);
            let _ = state.room.tx.send(server_msg);
        }
    }
    ControlFlow::Continue(())
}
