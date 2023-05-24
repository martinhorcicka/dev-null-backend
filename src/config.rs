use std::{fmt::Display, path::PathBuf, sync::Arc, time::Duration};

use notify::{EventKind, RecursiveMode, Watcher};
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

pub async fn watch_config(config: SharedConfig) -> ! {
    match load_config() {
        Ok(new_config) => *config.lock().await = new_config,
        Err(err) => println!(
            "Error loading configuration: {err:?}\nUsing default: {}",
            Config::default()
        ),
    }

    loop {
        println!("Starting config.json watch.");
        if tokio::spawn(watch_config_file(config.clone()))
            .await
            .err()
            .is_some()
        {
            println!("Watch config error. Restarting in 10 seconds..");
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}

async fn watch_config_file(config: SharedConfig) -> ! {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let mut config_watcher = notify::recommended_watcher(move |res| {
        if let Err(send_err) = tx.blocking_send(res) {
            println!("failed to send the event: {send_err:?}");
        }
    })
    .expect("config watcher created");

    let config_path = std::env::var("CONFIG_DIR").expect("CONFIG_DIR should be set");
    let mut config_path = PathBuf::from(config_path);
    config_path.push("config.json");

    config_watcher
        .watch(&config_path, RecursiveMode::Recursive)
        .expect("Config watching could not be established. Does config.json exist?");

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                if let EventKind::Modify(_) = event.kind {
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

    panic!("Stopping...");
}

fn load_config() -> Result<Config, Error> {
    let mut config_path = PathBuf::from(std::env::var("CONFIG_DIR").expect("config dir not set"));
    config_path.push("config.json");
    let config_string = std::fs::read_to_string(config_path)?;
    Ok(serde_json::from_str(&config_string)?)
}
