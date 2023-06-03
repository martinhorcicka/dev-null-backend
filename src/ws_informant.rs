use std::{collections::HashMap, time::Duration};

use tokio::sync::mpsc::{channel, Sender};

use crate::{error::Error, report};

#[derive(Debug, Clone)]
pub enum ServerInfoUpdate {
    MinecraftStatus(MinecraftStatus),
}

#[derive(Debug, Clone)]
pub enum MinecraftStatus {
    Online,
    Offline(String),
}

impl<T> From<Result<T, Error>> for MinecraftStatus {
    fn from(value: Result<T, Error>) -> Self {
        match value {
            Ok(_) => Self::Online,
            Err(error) => MinecraftStatus::Offline(error.to_string()),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct UniqueId(u128);
pub struct ServiceListener {
    sender: Sender<ServerInfoUpdate>,
}

impl ServiceListener {
    pub fn new(sender: Sender<ServerInfoUpdate>) -> Self {
        Self { sender }
    }
}

pub fn service_provider() -> Sender<ServiceListener> {
    let (tx, mut rx) = channel::<ServiceListener>(10);

    tokio::spawn(async move {
        let mut listeners: HashMap<UniqueId, ServiceListener> = HashMap::new();
        let mut counter = 0;
        let mut disconnected_sockets: Vec<UniqueId> = vec![];
        loop {
            for ds in &disconnected_sockets {
                listeners.remove(ds);
            }
            disconnected_sockets.clear();
            let task = check_mc_server_status();
            let receive_sender_task = rx.recv();

            tokio::select! {
                status = task => {
                    for l in &listeners {
                        let s = &l.1.sender;
                        if s.send(ServerInfoUpdate::MinecraftStatus(status.clone())).await.is_err() {
                            println!("websocket disconnected");
                            disconnected_sockets.push(*l.0);
                        }
                    }
                }
                recv = receive_sender_task => {
                    println!("received websocket connection");
                    if let Some(s) = recv {
                        let status = check_mc_server_status().await;
                        if s.sender.send(ServerInfoUpdate::MinecraftStatus(status)).await.is_err() {
                            println!("channel somehow died right after it was created");
                        };
                        listeners.insert(UniqueId(counter), s);
                        counter += 1;
                    }
                }
            }
        }
    });

    tx
}

async fn check_mc_server_status() -> MinecraftStatus {
    let status = report::mc::ping(1).into();
    if let MinecraftStatus::Online = status {
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    status
}
