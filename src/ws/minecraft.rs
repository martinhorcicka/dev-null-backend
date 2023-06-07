use std::{collections::HashMap, time::Duration};

use serde::Serialize;
use tokio::sync::mpsc;

use crate::{error::Error, report::mc::packet::ping::PingResponse};

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
            println!("failed to subscribe ws to MC job: {err}");
        }
    }

    pub async fn unsubscribe(&mut self, id: UniqueId) {
        if let Err(err) = self.unsubscribe_sender.send(id).await {
            println!("failed to unsubscribe ws to MC job: {err}");
        }
    }

    async fn service(
        mut sub_rx: mpsc::Receiver<(UniqueId, mpsc::Sender<SubscriptionResponse>)>,
        mut unsub_rx: mpsc::Receiver<UniqueId>,
    ) {
        let mut current_server_status = MinecraftServerStatus::default();

        let mut subscribers = HashMap::<UniqueId, mpsc::Sender<SubscriptionResponse>>::new();
        let mut ids_to_unsub = vec![];
        loop {
            for id in ids_to_unsub.iter() {
                subscribers.remove(id);
            }

            let sub_task = sub_rx.recv();
            let unsub_task = unsub_rx.recv();
            let sleep_task = async {
                let five_secs_later = tokio::time::Instant::now() + Duration::from_secs(5);
                let changed = current_server_status.apply_ping_response(crate::report::mc::ping(1));
                tokio::time::sleep_until(five_secs_later).await;
                changed
            };

            tokio::select! {
                new_sub = sub_task => {
                    if let Some((id, sender)) = new_sub {
                        println!("subbing {id:?} to minecraft");
                        if let Err(error) = sender
                            .send(SubscriptionResponse {
                                channel: crate::ws::management::Channel::Minecraft,
                                message: serde_json::to_string(&current_server_status.to_status_message())
                                    .expect("should always parse"),
                            })
                            .await
                        {
                            println!("failed to send mc message: {error}");
                        }

                        subscribers.insert(id,sender);
                    }

                },
                unsub = unsub_task => {
                    if let Some(id) = unsub {
                        println!("unsubbing {id:?} to minecraft");
                        subscribers.remove(&id);
                    }
                }
                changed = sleep_task => {
                    if changed {
                        for (id, sender) in subscribers.iter_mut() {
                            if let Err(error) = sender
                                .send(SubscriptionResponse {
                                    channel: crate::ws::management::Channel::Minecraft,
                                    message: serde_json::to_string(&current_server_status.to_status_message())
                                        .expect("should always parse"),
                                })
                                .await
                            {
                                println!("failed to send mc message: {error}, unsubbing");
                                ids_to_unsub.push(*id);
                            }
                        }
                    }
                }
            }
        }
    }
}

struct MinecraftServerStatus {
    online: bool,
    reason_for_offline: Option<String>,
}

impl Default for MinecraftServerStatus {
    fn default() -> Self {
        Self {
            online: false,
            reason_for_offline: Some("server status uninitialized".to_owned()),
        }
    }
}

impl MinecraftServerStatus {
    fn to_status_message(&self) -> StatusMessage {
        StatusMessage {
            online: self.online,
            reason: self.reason_for_offline.clone(),
        }
    }

    fn apply_ping_response(&mut self, response: Result<PingResponse, Error>) -> bool {
        let changed = self.online != response.is_ok();
        match response {
            Ok(_) => {
                self.online = true;
                self.reason_for_offline = None
            }
            Err(err) => {
                self.online = false;
                self.reason_for_offline = Some(err.to_string())
            }
        };
        changed
    }
}

#[derive(Debug, Serialize)]
struct StatusMessage {
    online: bool,
    reason: Option<String>,
}
