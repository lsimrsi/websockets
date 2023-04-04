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
use futures::{stream::StreamExt, SinkExt};

type SharedState = Arc<ServerState>;

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    name: String,
    message: String,
}

pub struct ServerState {
    messages: RwLock<Vec<ChatMessage>>,
    tx: broadcast::Sender<String>,
}

#[derive(Serialize)]
pub enum ServerMessageType {
    AllMessages,
    NewMessage,
}

/// Messages sent from server to client.
#[derive(Serialize)]
pub struct ServerMessage {
    msg_type: ServerMessageType,
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
        messages: RwLock::new(Vec::new()),
        tx,
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
        data: json!(*state.messages.read().await),
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

    let mut rx = state.tx.subscribe();

    // By splitting socket we can send and receive at the same time.
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            println!(
                "------------------------------------ send task msg: {}",
                msg
            );
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task to eceive messages from the client.
    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            if process_message(msg, who, state.clone()).await.is_break() {
                println!("Break received.");
                break;
            }
        }
        cnt
    });

    // If any one of the tasks run to completion, we abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    println!("Websocket context {} destroyed", who);
}

async fn process_message(msg: Message, who: SocketAddr, state: SharedState) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => match serde_json::from_str::<ChatMessage>(&t) {
            Ok(chat_msg) => {
                println!("{}: {}", chat_msg.name, chat_msg.message);
                let message = ServerMessage {
                    msg_type: ServerMessageType::NewMessage,
                    data: json!(&chat_msg),
                };
                state.messages.write().await.push(chat_msg);
                let _ = state.tx.send(serde_json::to_string(&message).unwrap());
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
