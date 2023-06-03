use std::time::Duration;

use serde::Serialize;
use tokio::{
    sync::mpsc::{channel, Receiver},
    time::Instant,
};

use crate::{error::Error, report};

use super::management::Notification;

#[derive(Debug, Clone, Serialize)]
pub struct MinecraftReport {
    pub online: bool,
    reason: Option<String>,
}

impl From<MinecraftReport> for Notification {
    fn from(value: MinecraftReport) -> Self {
        Notification::Minecraft(value)
    }
}

impl<T> From<Result<T, Error>> for MinecraftReport {
    fn from(value: Result<T, Error>) -> Self {
        match value {
            Ok(_) => MinecraftReport {
                online: true,
                reason: None,
            },
            Err(error) => MinecraftReport {
                online: false,
                reason: Some(error.to_string()),
            },
        }
    }
}

pub fn minecraft_job() -> Receiver<MinecraftReport> {
    let (sender, receiver) = channel(16);

    tokio::spawn(async move {
        loop {
            let one_second_later = Instant::now() + Duration::from_secs(1);
            let ping = report::mc::ping(1);
            if ping.is_ok() {
                tokio::time::sleep_until(one_second_later).await;
            }
            if sender.send(ping.into()).await.is_err() {
                println!("error sending report");
            }
        }
    });

    receiver
}
