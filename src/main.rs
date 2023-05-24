mod config;
mod error;
mod send_email;

use axum::{routing::post, Json, Router};
use config::{watch_config, Config, SharedConfig};
use send_email::{SendEmailData, SendEmailResponse};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenv::dotenv().ok();

    let state: SharedConfig = Config::default().into();

    let app = Router::new()
        .route("/send_email", post(send_email))
        .with_state(state.clone());

    tokio::spawn(watch_config(state));

    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn send_email(Json(send_email_data): Json<SendEmailData>) -> Json<SendEmailResponse> {
    Json(send_email::send_email(send_email_data))
}
