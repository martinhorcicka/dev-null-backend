use std::collections::{HashMap, HashSet};

use serde::Serialize;
use tokio::sync::mpsc;

use crate::ws::minecraft::MinecraftJob;

#[derive(Debug)]
pub enum Command {
    None,
    Register(mpsc::Sender<SubscriptionResponse>),
    Subscribe(UniqueId, Channel),
    Unsubscribe(UniqueId, Channel),
    Unregister(UniqueId),
}

#[derive(Debug)]
pub enum CommandResponse {
    None,
    Registered(UniqueId),
    Subscribed,
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
    message: SubscriptionResponseMessage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum SubscriptionResponseMessage {
    Minecraft(super::minecraft::StatusMessage),
}

impl From<super::minecraft::StatusMessage> for SubscriptionResponse {
    fn from(value: super::minecraft::StatusMessage) -> Self {
        SubscriptionResponse {
            channel: Channel::Minecraft,
            message: SubscriptionResponseMessage::Minecraft(value),
        }
    }
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
            tracing::warn!("Error sending command to manager service: {error}");
        }

        rx.recv().await
    }
}

async fn manager_service(
    mut command_receiver: mpsc::Receiver<(Command, mpsc::Sender<CommandResponse>)>,
) {
    let mut mc_job = MinecraftJob::new();

    let mut current_unique_id = UniqueId(0);
    let mut registered_sockets = HashSet::<UniqueId>::new();
    let mut registered_senders = HashMap::<UniqueId, mpsc::Sender<SubscriptionResponse>>::new();
    loop {
        if let Some((command, tx)) = command_receiver.recv().await {
            if let Err(send_error) = match command {
                Command::Register(sender) => {
                    let id = current_unique_id.next();
                    tracing::debug!("registering websocket id {id:?}");
                    registered_sockets.insert(id);
                    registered_senders.insert(id, sender);
                    tx.send(CommandResponse::Registered(id))
                }
                Command::Unregister(id) => {
                    tracing::debug!("unregistering websocket with id {id:?}");
                    registered_sockets.remove(&id);
                    registered_senders.remove(&id);
                    tx.send(CommandResponse::Unregistered)
                }
                Command::Subscribe(id, channel) => {
                    tracing::debug!("subscribing {id:?} to {channel:?}");
                    match channel {
                        Channel::Minecraft => {
                            mc_job.subscribe(id, registered_senders[&id].clone()).await
                        }
                    }
                    tx.send(CommandResponse::Subscribed)
                }
                Command::Unsubscribe(id, channel) => {
                    tracing::debug!("unsubscribing {id:?} from {channel:?}");
                    match channel {
                        Channel::Minecraft => mc_job.unsubscribe(id).await,
                    }
                    tx.send(CommandResponse::Unsubscribed)
                }
                Command::None => tx.send(CommandResponse::None),
            }
            .await
            {
                tracing::warn!("error sending response back to websocket: {send_error}");
            }
        } else {
            tracing::error!("channel has been closed, stopping the service..");
            break;
        }
    }
    tracing::debug!("manager_service stopped");
}
