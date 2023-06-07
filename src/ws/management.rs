use std::collections::HashSet;

use serde::Serialize;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub enum Command {
    None,
    Register,
    Subscribe(UniqueId, Channel),
    Unsubscribe(UniqueId, Channel),
    Unregister(UniqueId),
}

#[derive(Debug)]
pub enum CommandResponse {
    None,
    Registered(UniqueId),
    Subscribed(broadcast::Receiver<SubscriptionResponse>),
    Unsubscribed,
    Unregistered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniqueId(u128);
impl UniqueId {
    fn next(&mut self) -> UniqueId {
        let old = *self;
        self.0 += 1;
        old
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Minecraft,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionResponse {
    pub channel: Channel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Manager {
    command_sender: mpsc::Sender<(Command, mpsc::Sender<CommandResponse>)>,
}

impl Manager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(16);
        tokio::spawn(manager_service(rx));
        Self { command_sender: tx }
    }

    pub async fn send_command(&self, command: Command) -> Option<CommandResponse> {
        let (tx, mut rx) = mpsc::channel(1);
        if let Err(error) = self.command_sender.send((command, tx)).await {
            println!("Error sending command to manager service: {error}");
        }

        rx.recv().await
    }
}

async fn manager_service(
    mut command_receiver: mpsc::Receiver<(Command, mpsc::Sender<CommandResponse>)>,
) {
    let mut current_unique_id = UniqueId(0);
    let mut registered_sockets = HashSet::<UniqueId>::new();
    let (sub_tx, _sub_rx) = broadcast::channel(128);
    loop {
        if let Some((command, tx)) = command_receiver.recv().await {
            println!("received command {command:?}");
            if let Err(send_error) = match command {
                Command::Register => {
                    let id = current_unique_id.next();
                    println!("registering websocket id {id:?}");
                    registered_sockets.insert(id);
                    tx.send(CommandResponse::Registered(id))
                }
                Command::Unregister(id) => {
                    println!("unregistering websocket with id {id:?}");
                    registered_sockets.remove(&id);
                    tx.send(CommandResponse::Unregistered)
                }
                Command::Subscribe(id, channel) => {
                    println!("subscribing {id:?} to {channel:?}");
                    tx.send(CommandResponse::Subscribed(sub_tx.subscribe()))
                }
                Command::Unsubscribe(id, channel) => {
                    println!("unsubscribing {id:?} from {channel:?}");
                    tx.send(CommandResponse::Unsubscribed)
                }
                Command::None => tx.send(CommandResponse::None),
            }
            .await
            {
                println!("error sending response back to websocket: {send_error}");
            }
        } else {
            println!("channel has been closed, stopping the service..");
            break;
        }
    }
    println!("manager_service stopped");
}
