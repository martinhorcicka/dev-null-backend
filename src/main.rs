mod error;
mod report;
mod send_email;

use axum::{
    routing::{get, post},
    Json, Router,
};
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

    let app = Router::new()
        .route("/send_email", post(send_email))
        .route("/report/mc/ping/:payload", get(report::mc::ping_route))
        .route("/report/mc/status", get(report::mc::status_route));

    let ip = std::env::var("BACKEND_ADDR").expect("cannot run without specified address");
    let port = std::env::var("BACKEND_PORT").expect("cannot run without specified port");
    let addr = format!("{ip}:{port}").parse().expect("invalid format for ip and/or port");
    tracing::debug!("listening on {addr}");
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn send_email(Json(send_email_data): Json<SendEmailData>) -> Json<SendEmailResponse> {
    Json(send_email::send_email(send_email_data))
}
