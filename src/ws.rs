use std::{net::SocketAddr, ops::ControlFlow};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    headers,
    response::IntoResponse,
    TypedHeader,
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::ws_informant::{MinecraftStatus, ServerInfoUpdate, ServiceListener};

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(sender): State<Sender<ServiceListener>>,
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
    ws.on_upgrade(move |socket| handle_socket(socket, addr, sender))
}

#[derive(Debug, Serialize)]
struct McServerStatus {
    online: bool,
    reason: Option<String>,
}

impl From<MinecraftStatus> for McServerStatus {
    fn from(value: MinecraftStatus) -> Self {
        let (online, reason) = match value {
            MinecraftStatus::Online => (true, None),
            MinecraftStatus::Offline(err) => (false, Some(err)),
        };
        Self { online, reason }
    }
}

async fn server_communication(
    mut receiver: Receiver<ServerInfoUpdate>,
    mut sender: SplitSink<WebSocket, Message>,
    who: SocketAddr,
) {
    let mut previous_status = false;
    while let Some(info) = receiver.recv().await {
        match info {
            ServerInfoUpdate::MinecraftStatus(mc_status) => {
                let server_status: McServerStatus = mc_status.into();

                if previous_status != server_status.online {
                    previous_status = server_status.online;
                    let response = serde_json::to_string(&server_status)
                        .expect("this parse should always succeed");

                    if sender.send(Message::Text(response)).await.is_err() {
                        println!("client {who} abruptly disconnected");
                    }
                }
            }
        }
    }
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr, sender: Sender<ServiceListener>) {
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {}...", who);
    } else {
        println!("Could not send ping {}!", who);
        return;
    }

    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who).is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }

    let (tx, rx) = channel(10);
    println!("notifying server about new websocket connection..");
    if sender.send(ServiceListener::new(tx)).await.is_err() {
        println!("couldn't reach the server, closing connection..");
        return;
    }

    let (sender, mut receiver) = socket.split();
    let mut send_task = tokio::spawn(server_communication(rx, sender, who));

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(msg, who).is_break() {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => {
            println!("server communication failed to join back to the caller thread");
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    println!("websocket context {who} destroyed");
}

fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    "{} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!("{} somehow sent close message without CloseFrame", who);
            }
            ControlFlow::Break(())
        }
        _ => ControlFlow::Continue(()),
    }
}
