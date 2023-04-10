use axum::extract::ws::{Message, WebSocket};
use serde_json::json;
use tokio::sync::mpsc::{self, Sender};

use std::net::SocketAddr;
use std::ops::ControlFlow;

use futures::{stream::StreamExt, SinkExt};

use crate::{
    ChatMessage, ClientMessage, ClientMessageType, ServerMessage, ServerMessageType, SharedState,
};

pub struct Client {
    name: String,
    socket: WebSocket,
    who: SocketAddr,
    state: SharedState,
}

impl Client {
    pub fn new(socket: WebSocket, who: SocketAddr, state: SharedState) -> Client {
        Client {
            name: "".to_string(),
            socket,
            who,
            state,
        }
    }

    /// Actual websocket statemachine (one will be spawned per connection).
    pub async fn listen(mut self) {
        // Send a ping (unsupported by some browsers) to kick things off.
        if self.socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
            println!("<<< Pinged {}...", self.who);
        } else {
            println!("Could not send ping to {}.", self.who);
            return;
        }

        let all_messages = ServerMessage {
            msg_type: ServerMessageType::AllMessages,
            data: json!(*self.state.room.state.read().await.messages),
        };

        let all_messages = match serde_json::to_string(&all_messages) {
            Ok(s) => s,
            Err(_) => {
                println!("Could not convert all messages to string");
                return;
            }
        };

        if !self.socket.send(Message::Text(all_messages)).await.is_ok() {
            println!("Could not send all messages to client.");
            return;
        };

        let mut rx = self.state.room.tx.subscribe();

        let (mut sink, mut stream) = self.socket.split();
        let (sender, mut receiver) = mpsc::channel::<ServerMessage>(16);

        // Forwards messages from mpsc to sink.
        let mut forward_task = tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                println!("<<<--- {}: {:?}", self.who, message);
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
                    if process_message(msg, &mut stream_sender, self.who, self.state.clone())
                        .await
                        .is_break()
                    {
                        println!("Break received for {}.", self.who);
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

        println!("Websocket context {} destroyed", self.who);
    }
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
            Err(_) => println!("--->>> {} sent str: {:?}", who, t),
        },
        Message::Binary(d) => {
            println!("--->>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(
                    "--->>> {} somehow sent close message without CloseFrame",
                    who
                );
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!("--->>> {} sent pong with {:?}", who, v);
        }
        // You should never need to manually handle Message::Ping.
        // Axum's websocket library will do so automagically.
        Message::Ping(v) => {
            println!("--->>> {} sent ping with {:?}", who, v);
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
    println!("--->>> client message from {}: {:?}", who, client_msg);
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
                let room_users = &state.room.state.read().await.users;

                if room_users.contains(&name) {
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
                users.push(name.clone());
            }

            let server_msg = ServerMessage {
                msg_type: ServerMessageType::NameRegistered,
                data: json!(""),
            };
            if sender.send(server_msg.clone()).await.is_err() {
                return ControlFlow::Break(());
            };
            let server_msg = ServerMessage {
                msg_type: ServerMessageType::Joined,
                data: json!(format!("{} joined.", name)),
            };
            if state.room.tx.send(server_msg).is_err() {
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
