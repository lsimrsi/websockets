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
use tokio::sync::{broadcast, RwLock};

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
use futures::{
    stream::{SplitSink, StreamExt},
    SinkExt,
};

type SharedState = Arc<ServerState>;

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    name: String,
    message: String,
}

pub struct Room {
    state: RwLock<RoomState>,
    tx: broadcast::Sender<String>,
}

impl Room {
    pub fn new(tx: broadcast::Sender<String>) -> Room {
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

#[derive(Serialize, Debug)]
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
#[derive(Serialize, Debug)]
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

    let (tx, _rx) = broadcast::channel(100);
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let shared_state = Arc::new(ServerState {
        room: Room::new(tx),
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
async fn handle_socket(mut socket: WebSocket, who: SocketAddr, state: SharedState) {
    //send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("<<< Pinged {}...", who);
    } else {
        println!("Could not send ping to {}!", who);
        // No error here since the only thing we can do is close the connection.
        // If we can not send messages, there is no way to salvage the state machine anyway.
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

    // while let Ok(msg) = rx.recv().await {
    //     println!(
    //         "------------------------------------ send task msg: {}",
    //         msg
    //     );
    //     // In any websocket error, break loop.
    //     if sender.send(Message::Text(msg)).await.is_err() {
    //         break;
    //     }
    // }

    // By splitting socket we can send and receive at the same time.
    // let (mut sender, mut receiver) = socket.split();

    // let sender_send_task = sender.clone();
    // let mut send_task = tokio::spawn(async move {
    //     while let Ok(msg) = rx.recv().await {
    //         println!("--------------- send task msg: {}", msg);
    //         // In any websocket error, break loop.
    //         if sender.send(Message::Text(msg)).await.is_err() {
    //             break;
    //         }
    //     }
    // });

    // Task to receive messages from the client.
    // let mut recv_task = tokio::spawn(async move {
    //     loop {
    //         if let Some(Ok(msg)) = receiver.next().await {
    //             if process_message(msg, &mut sender, who, state.clone())
    //                 .await
    //                 .is_break()
    //             {
    //                 println!("Break received for {}.", who);
    //                 break;
    //             }
    //         }
    //     }
    // });

    loop {
        println!("waiting for rx");
        if let Ok(msg) = rx.try_recv() {
            println!(
                "------------------------------------ send task msg: {}",
                msg
            );
            // In any websocket error, break loop.
            if socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
        if let Some(Ok(msg)) = socket.recv().await {
            println!("waiting for process msg");
            if process_message(msg, &mut socket, who, state.clone())
                .await
                .is_break()
            {
                println!("Break received for {}.", who);
                break;
            }
        }
    }

    // If any one of the tasks run to completion, we abort the other.
    // tokio::select! {
    //     // _ = (&mut send_task) => recv_task.abort(),
    //     _ = (&mut recv_task) => recv_task.abort(),
    // };

    println!("Websocket context {} destroyed", who);
}

async fn process_message(
    msg: Message,
    socket: &mut WebSocket,
    who: SocketAddr,
    state: SharedState,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => match serde_json::from_str::<ClientMessage>(&t) {
            Ok(client_msg) => {
                return process_client_message(client_msg, socket, who, state).await;
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
    socket: &mut WebSocket,
    who: SocketAddr,
    state: SharedState,
) -> ControlFlow<(), ()> {
    println!(">>> client message: {:?}", client_msg);
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
                    if socket
                        .send(Message::Text(json!(&server_msg).to_string()))
                        .await
                        .is_err()
                    {
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
            if socket
                .send(Message::Text(json!(&server_msg).to_string()))
                .await
                .is_err()
            {
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
            let _ = state.room.tx.send(json!(&server_msg).to_string());
        }
    }
    ControlFlow::Continue(())
}
