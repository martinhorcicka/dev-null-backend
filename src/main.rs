mod config;
mod error;

use axum::{routing::get, Router};
use config::{watch_config, Config, SharedConfig};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let state: SharedConfig = Config::default().into();

    let app = Router::new()
        .route("/", get(root))
        .with_state(state.clone());

    tokio::spawn(watch_config(state));

    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello!"
}
