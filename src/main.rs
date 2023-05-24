mod config;
mod error;
mod send_email;

use std::net::SocketAddr;

use axum::{extract::State, routing::post, Json, Router};
use config::{watch_config, Config, SharedConfig};
use send_email::{SendEmailData, SendEmailResponse};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dev-null_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv::dotenv().ok();

    let state: SharedConfig = Config::default().into();

    let app = Router::new()
        .route("/send_email", post(send_email))
        .with_state(state.clone());

    tokio::spawn(watch_config(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::debug!("listening on {addr}");
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn send_email(
    State(config): State<SharedConfig>,
    Json(send_email_data): Json<SendEmailData>,
) -> Json<SendEmailResponse> {
    Json(send_email::send_email(
        send_email_data,
        config.lock().await.clone(),
    ))
}
