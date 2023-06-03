use std::collections::HashMap;

use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::minecraft::{self, MinecraftReport};

#[derive(Debug, Clone)]
pub struct WsManager {
    sender: Sender<SubscriptionInfo>,
}

impl WsManager {
    pub fn new() -> Self {
        let (sender, receiver) = channel(10);
        tokio::spawn(ws_manager_service(receiver));
        WsManager { sender }
    }

    pub async fn subscribe(&self, info: SubscriptionInfo) {
        if self.sender.send(info).await.is_err() {
            println!("failed to subscribe a websocket");
        }
    }
}

#[derive(Debug)]
pub struct SubscriptionInfo {
    channel: SubscriptionChannel,
    sender: Sender<Notification>,
}

impl SubscriptionInfo {
    pub fn new(channel: SubscriptionChannel, sender: Sender<Notification>) -> Self {
        Self { channel, sender }
    }
}

#[derive(Clone)]
pub enum Notification {
    Minecraft(MinecraftReport),
}

#[derive(Debug, PartialEq)]
pub enum SubscriptionChannel {
    Minecraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UniqueId(u128);
impl UniqueId {
    fn inc(&mut self) {
        self.0 += 1;
    }
}

async fn ws_manager_service(mut sub_receiver: Receiver<SubscriptionInfo>) {
    let mut counter = UniqueId(0);
    let mut websockets = HashMap::<UniqueId, Sender<Notification>>::new();
    let mut subscribed_channels = HashMap::<UniqueId, SubscriptionChannel>::new();
    let mut ids_marked_for_deletion = vec![];

    let mut mc_recv = minecraft::minecraft_job();

    loop {
        for id in ids_marked_for_deletion.iter() {
            websockets.remove(id);
            subscribed_channels.remove(id);
        }
        ids_marked_for_deletion.clear();

        let mc_recv_job = mc_recv.recv();
        let sub_recv_job = sub_receiver.recv();

        tokio::select! {
            mc_report = mc_recv_job => {
                if let Some(report) = mc_report {
                    report_to_subscribers(report.into(), &mut websockets, &subscribed_channels, &mut ids_marked_for_deletion).await;
                }
            },
            sub_info = sub_recv_job => {
                println!("sub_info: {sub_info:?}");
                if let Some(info) = sub_info {
                    subscribed_channels.insert(counter, info.channel);
                    websockets.insert(counter, info.sender);
                    counter.inc();
                }
            }
        }
    }
}

async fn report_to_subscribers(
    report: Notification,
    websockets: &mut HashMap<UniqueId, Sender<Notification>>,
    subbed_channels: &HashMap<UniqueId, SubscriptionChannel>,
    ids_marked_for_deletion: &mut Vec<UniqueId>,
) {
    for id in subbed_channels.iter().filter_map(|(id, ch)| {
        if *ch
            == match &report {
                Notification::Minecraft(_) => SubscriptionChannel::Minecraft,
            }
        {
            Some(id)
        } else {
            None
        }
    }) {
        if let Some(ws) = websockets.get(id) {
            if ws.send(report.clone()).await.is_err() {
                ids_marked_for_deletion.push(*id);
            }
        }
    }
}
