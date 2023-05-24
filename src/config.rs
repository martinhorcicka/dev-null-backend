use std::{fmt::Display, path::Path, sync::Arc};

use notify::{event::ModifyKind, EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::error::Error;

pub type SharedConfig = Arc<Mutex<Config>>;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "receiverEmail")]
    pub receiver_email: String,
    #[serde(rename = "emailSubject")]
    pub email_subject: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            receiver_email: "info-dev-null@email.cz".to_owned(),
            email_subject: "Commision".to_owned(),
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{receiverEmail: {}, emailSubject: {} }}",
            self.receiver_email, self.email_subject
        )
    }
}

impl From<Config> for SharedConfig {
    fn from(value: Config) -> Self {
        Arc::new(Mutex::new(value))
    }
}

pub async fn watch_config(config: SharedConfig) {
    match load_config() {
        Ok(new_config) => *config.lock().await = new_config,
        Err(err) => println!(
            "Error loading configuration: {err:?}\nUsing default: {}",
            Config::default()
        ),
    }
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let mut config_watcher = notify::recommended_watcher(move |res| {
        if let Err(send_err) = tx.blocking_send(res) {
            println!("failed to send the event: {send_err:?}");
        }
    })
    .expect("config watcher created");

    config_watcher
        .watch(Path::new("config.json"), RecursiveMode::Recursive)
        .expect("Config watching could not be established. Does config.json exist?");

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                if let EventKind::Modify(ModifyKind::Any) = event.kind {
                    match load_config() {
                        Ok(new_config) => {
                            println!("Updating config with {new_config}");
                            *config.lock().await = new_config;
                        }
                        Err(err) => println!("Error loading configuration: {err:?}"),
                    }
                }
            }
            Err(err) => println!("Error: {err:?}"),
        }
    }
}

fn load_config() -> Result<Config, Error> {
    let config_string = std::fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&config_string)?)
}
