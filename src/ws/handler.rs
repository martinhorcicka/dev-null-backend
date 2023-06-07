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
use futures_util::StreamExt;

use super::management::Manager;

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
    ping_websocket(&mut socket, &who).await;

    if let Some(response) = manager
        .send_command(super::management::Command::Register)
        .await
    {
        let id = match response {
            crate::ws::management::CommandResponse::Registered(id) => id,
            _ => return,
        };

        handle_communication_with_manager(socket, &manager).await;

        manager
            .send_command(super::management::Command::Unregister(id))
            .await;
    }

    println!("websocket context {who} destroyed");
}

async fn handle_communication_with_manager(socket: WebSocket, manager: &Manager) {
    let (tx, mut rx) = socket.split();

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = rx.next().await {
            if process_message(msg).is_break() {
                break;
            }
        }
    });
}

fn process_message(msg: Message) -> ControlFlow<(), ()> {
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
        _ => ControlFlow::Continue(()),
    }
}

async fn ping_websocket(socket: &mut WebSocket, who: &SocketAddr) {
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {}...", who);
    } else {
        println!("Could not send ping {}!", who);
        return;
    }

    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg).is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }
}
