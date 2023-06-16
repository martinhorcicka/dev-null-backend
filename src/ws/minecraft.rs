use std::{collections::HashMap, time::Duration};

use serde::Serialize;
use tokio::sync::mpsc;

use crate::{
    error::Error,
    report::mc::packet::{ping::PingResponse, status::StatusResponse},
};

use super::management::{SubscriptionResponse, UniqueId};

pub struct MinecraftJob {
    subscribe_sender: mpsc::Sender<(UniqueId, mpsc::Sender<SubscriptionResponse>)>,
    unsubscribe_sender: mpsc::Sender<UniqueId>,
}

impl MinecraftJob {
    pub fn new() -> Self {
        let (sub_tx, sub_rx) = mpsc::channel(32);
        let (unsub_tx, unsub_rx) = mpsc::channel(32);

        tokio::spawn(MinecraftJob::service(sub_rx, unsub_rx));

        MinecraftJob {
            subscribe_sender: sub_tx,
            unsubscribe_sender: unsub_tx,
        }
    }

    pub async fn subscribe(&mut self, id: UniqueId, sender: mpsc::Sender<SubscriptionResponse>) {
        if let Err(err) = self.subscribe_sender.send((id, sender)).await {
            tracing::warn!("failed to subscribe ws to MC job: {err}");
        }
    }

    pub async fn unsubscribe(&mut self, id: UniqueId) {
        if let Err(err) = self.unsubscribe_sender.send(id).await {
            tracing::warn!("failed to unsubscribe ws to MC job: {err}");
        }
    }

    async fn service(
        mut sub_rx: mpsc::Receiver<(UniqueId, mpsc::Sender<SubscriptionResponse>)>,
        mut unsub_rx: mpsc::Receiver<UniqueId>,
    ) {
        let mut current_server_status = MinecraftServerStatus::default();

        let mut subscribers = HashMap::<UniqueId, mpsc::Sender<SubscriptionResponse>>::new();
        let mut ids_to_unsub = vec![];

        let (update_sender, mut update_receiver) = mpsc::channel(16);
        let ping_update_sender = update_sender.clone();
        let ping_task = async move {
            loop {
                let five_secs_later = tokio::time::Instant::now() + Duration::from_secs(5);
                let ping_response = crate::report::mc::ping(1);
                if let Err(err) = ping_update_sender
                    .send(ping_response.map(ResponseMessage::Ping))
                    .await
                {
                    tracing::error!("failed to send ping message `{err}`");
                }
                tokio::time::sleep_until(five_secs_later).await;
            }
        };
        tokio::spawn(ping_task);

        let status_update_sender = update_sender.clone();
        let status_task = async move {
            loop {
                let one_minute_later = tokio::time::Instant::now() + Duration::from_secs(60);
                let status_response = crate::report::mc::status();
                if let Err(err) = status_update_sender
                    .send(status_response.map(ResponseMessage::Status))
                    .await
                {
                    tracing::error!("failed to send status message `{err}`");
                }
                tokio::time::sleep_until(one_minute_later).await;
            }
        };
        tokio::spawn(status_task);

        loop {
            for id in ids_to_unsub.iter() {
                subscribers.remove(id);
            }
            ids_to_unsub.clear();

            let sub_task = sub_rx.recv();
            let unsub_task = unsub_rx.recv();

            let update_task = update_receiver.recv();

            tokio::select! {
                new_sub = sub_task => {
                    if let Some((id, sender)) = new_sub {
                        tracing::debug!("subbing {id:?} to minecraft");
                        if let Err(error) = sender
                            .send(current_server_status.to_update_message().into())
                            .await
                        {
                            tracing::warn!("failed to send mc message: {error}");
                        }

                        subscribers.insert(id,sender);
                    }

                },
                unsub = unsub_task => {
                    if let Some(id) = unsub {
                        tracing::debug!("unsubbing {id:?} from minecraft");
                        subscribers.remove(&id);
                    }
                }
                message = update_task => {
                    let update_message = match message {
                        None => {tracing::error!("mc update channel closed.."); return},
                        Some(message) => current_server_status.apply_response(message)
                    };
                    if let Some(update_message) = update_message {
                        for (id, sender) in subscribers.iter_mut() {
                            if let Err(error) = sender
                                .send(update_message.clone().into())
                                .await
                            {
                                tracing::warn!("failed to send mc message: {error}, unsubbing");
                                ids_to_unsub.push(*id);
                            }
                        }
                    }
                }
            }
        }
    }
}

enum ResponseMessage {
    Ping(PingResponse),
    Status(StatusResponse),
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftServerStatus {
    online: bool,
    reason_for_offline: Option<String>,
    description: String,
    players: Players,
    version: String,
    favicon: String,
    mods: HashMap<String, String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Players {
    online: i32,
    max: i32,
}

impl Default for MinecraftServerStatus {
    fn default() -> Self {
        Self {
            online: false,
            reason_for_offline: Some("server status uninitialized".to_owned()),
            description: String::new(),
            players: Players { online: 0, max: 20 },
            version: "0.0.0".to_owned(),
            favicon: String::new(),
            mods: HashMap::new(),
        }
    }
}

impl MinecraftServerStatus {
    fn to_update_message(&self) -> UpdateMessage {
        UpdateMessage::Initial(self.clone())
    }

    fn to_status_message(&self) -> UpdateMessage {
        UpdateMessage::Status(StatusMessage {
            online: self.online,
            reason_for_offline: self.reason_for_offline.clone(),
        })
    }

    fn to_online_players(&self) -> UpdateMessage {
        UpdateMessage::OnlinePlayers(OnlinePlayersMessage {
            players_online: self.players.online,
        })
    }

    fn apply_response(
        &mut self,
        response: Result<ResponseMessage, Error>,
    ) -> Option<UpdateMessage> {
        match response {
            Err(err) => {
                let changed = self.online;
                self.online = false;
                self.reason_for_offline = Some(err.to_string());

                if changed {
                    Some(self.to_status_message())
                } else {
                    None
                }
            }
            Ok(response) => match response {
                ResponseMessage::Ping(_ping) => {
                    let changed = !self.online;
                    self.online = true;
                    self.reason_for_offline = None;
                    if changed {
                        Some(self.to_status_message())
                    } else {
                        None
                    }
                }
                ResponseMessage::Status(status) => self.apply_status_response(status),
            },
        }
    }

    fn apply_status_response(&mut self, status: StatusResponse) -> Option<UpdateMessage> {
        let online_players_changed = self.players.online != status.players.online;

        *self = MinecraftServerStatus {
            online: true,
            reason_for_offline: None,
            description: status.description.text,
            players: status.players.into(),
            version: status.version.name,
            favicon: status.favicon,
            mods: status.forge_data.d.mods,
        };

        if online_players_changed {
            Some(self.to_online_players())
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UpdateMessage {
    Initial(MinecraftServerStatus),
    Status(StatusMessage),
    OnlinePlayers(OnlinePlayersMessage),
}

impl From<crate::report::mc::packet::status::Players> for Players {
    fn from(value: crate::report::mc::packet::status::Players) -> Self {
        Self {
            online: value.online,
            max: value.max,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatusMessage {
    online: bool,
    reason_for_offline: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OnlinePlayersMessage {
    players_online: i32,
}
