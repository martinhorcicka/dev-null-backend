use std::{collections::HashMap, net::SocketAddr, ops::ControlFlow};

use crate::ws::management::CommandResponse;

use super::management::{Manager, SubscriptionResponse, UniqueId};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    headers,
    response::IntoResponse,
    TypedHeader,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(manager): State<Manager>,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, manager))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr, manager: Manager) {
    if ping_websocket(&mut socket, &who).await.is_break() {
        return;
    }

    if let Some(response) = manager
        .send_command(super::management::Command::Register)
        .await
    {
        let id = match response {
            crate::ws::management::CommandResponse::Registered(id) => id,
            _ => return,
        };

        handle_communication_with_manager(socket, id, &manager).await;

        manager
            .send_command(super::management::Command::Unregister(id))
            .await;
    }

    println!("websocket context {who} destroyed");
}

async fn handle_communication_with_manager(socket: WebSocket, id: UniqueId, manager: &Manager) {
    let (mut tx, mut rx) = socket.split();
    let mut listening_channels =
        HashMap::<Channel, broadcast::Receiver<SubscriptionResponse>>::new();

    let mut send_task = tokio::spawn(async move {
        loop {
            for (_ch, recv) in listening_channels.iter_mut() {
                if let Ok(response) = recv.recv().await {
                    if let Err(error) = tx
                        .send(Message::Text(
                            serde_json::to_string(&response)
                                .expect("should always parse successfully"),
                        ))
                        .await
                    {
                        println!("failed sending info to websocket: {error}");
                    }
                }
            }
        }
    });

    let mgr = manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = rx.next().await {
            if let ControlFlow::Continue(command) = process_message(msg) {
                if let Some(CommandResponse::Subscribed(_recv)) =
                    mgr.send_command(command.with_unique_id(id)).await
                {
                    match command {
                        WebsocketCommand::Known {
                            command: _,
                            channel,
                        } => {
                            println!("trying to assign a receiver to {channel:?} channel");
                        }
                        WebsocketCommand::Unknown => {}
                    }
                }
            } else {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
enum WebsocketCommand {
    Known { command: Command, channel: Channel },
    Unknown,
}

impl WebsocketCommand {
    fn with_unique_id(&self, id: UniqueId) -> super::management::Command {
        match *self {
            WebsocketCommand::Known { command, channel } => match command {
                Command::Subscribe => super::management::Command::Subscribe(id, channel.into()),
                Command::Unsubscribe => super::management::Command::Unsubscribe(id, channel.into()),
            },
            WebsocketCommand::Unknown => super::management::Command::None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Command {
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Channel {
    Minecraft,
}

impl From<Channel> for super::management::Channel {
    fn from(value: Channel) -> Self {
        match value {
            Channel::Minecraft => Self::Minecraft,
        }
    }
}

fn process_message(msg: Message) -> ControlFlow<(), WebsocketCommand> {
    match msg {
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    "websocket sent close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!("somehow sent close message without CloseFrame");
            }
            ControlFlow::Break(())
        }
        Message::Text(text) => {
            let command = match serde_json::from_str::<WebsocketCommand>(&text) {
                Ok(cmd) => cmd,
                Err(_) => WebsocketCommand::Unknown,
            };
            ControlFlow::Continue(command)
        }
        _ => ControlFlow::Continue(WebsocketCommand::Unknown),
    }
}

async fn ping_websocket(
    socket: &mut WebSocket,
    who: &SocketAddr,
) -> ControlFlow<(), WebsocketCommand> {
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {}...", who);
    } else {
        println!("Could not send ping {}!", who);
        return ControlFlow::Break(());
    }

    if let Some(Ok(msg)) = socket.recv().await {
        process_message(msg)
    } else {
        println!("client {who} abruptly disconnected");
        ControlFlow::Break(())
    }
}
