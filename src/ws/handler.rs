use std::{net::SocketAddr, ops::ControlFlow, time::Duration};

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
use serde::Deserialize;
use tokio::sync::mpsc::{channel, Receiver};

use crate::ws::management::{SubscriptionChannel, SubscriptionInfo};

use super::management::{Notification, WsManager};

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(manager): State<WsManager>,
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

async fn server_communication(
    mut receiver: Receiver<Notification>,
    mut sender: SplitSink<WebSocket, Message>,
) {
    let mut previous_mc_server_status: Option<bool> = None;
    while let Some(info) = receiver.recv().await {
        match info {
            Notification::Minecraft(mc_report) => {
                if previous_mc_server_status.is_none()
                    || previous_mc_server_status.is_some_and(|s| s != mc_report.online)
                {
                    if sender
                        .send(Message::Text(serde_json::to_string(&mc_report).unwrap()))
                        .await
                        .is_err()
                    {
                        println!("failed to send to websocket");
                    }

                    previous_mc_server_status = Some(mc_report.online);
                }
            }
        }
    }
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr, manager: WsManager) {
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

    let client_request = socket.recv();
    let timeout = tokio::time::sleep(Duration::from_secs(60));
    let client_response = tokio::select! {
        req = client_request => req,
        _ = timeout => None
    };
    let sub_request: Option<SubscriptionChannel> =
        if let Some(Ok(Message::Text(msg))) = client_response {
            serde_json::from_str::<SubscriptionRequest>(&msg)
                .ok()
                .and_then(|s| s.try_into().ok())
        } else {
            println!("websocket didn't specify a subscription in 1 minute since connecting");
            return;
        };

    if let Some(sub_channel) = sub_request {
        let (sender, receiver) = channel(16);
        manager
            .subscribe(SubscriptionInfo::new(sub_channel, sender))
            .await;

        let (s, mut r) = socket.split();

        let mut sending_task = tokio::spawn(server_communication(receiver, s));
        let mut receiving_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = r.next().await {
                if process_message(msg, who).is_break() {
                    break;
                }
            }
        });

        tokio::select! {
            _ = (&mut sending_task) => {
                receiving_task.abort();
            },
            _ = (&mut receiving_task) => {
                println!("socket sent a close message");
                sending_task.abort();
            }
        }
    }

    println!("websocket context {who} destroyed");
}

#[derive(Deserialize)]
struct SubscriptionRequest {
    channel: String,
}

impl TryFrom<SubscriptionRequest> for SubscriptionChannel {
    type Error = String;

    fn try_from(value: SubscriptionRequest) -> Result<Self, Self::Error> {
        if value.channel == "minecraft" {
            Ok(SubscriptionChannel::Minecraft)
        } else {
            Err("invalid channel name".to_owned())
        }
    }
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
