mod routes;
mod state;
mod template;
mod utils;
mod yt_dlp;

use crate::routes::{download, root};
use crate::state::{CONFIG, RunTimeState};
use axum::{BoxError, Router, error_handling::HandleErrorLayer, http::StatusCode, routing::get};
use axum::{http::Request, response::Response};
use std::fs;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tower::{ServiceBuilder, buffer::BufferLayer, limit::RateLimitLayer};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{Span, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn setup_env() {
    if dotenvy::dotenv().is_ok() {
        println!("loaded env");
    } else {
        eprintln!("failed to load env");
    }
}

fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    info!("logging setup done")
}

fn setup_files_dir() {
    if std::path::Path::exists(&CONFIG.root_dir) {
        info!("files directory exists, deleting it.");
        fs::remove_dir_all(&CONFIG.root_dir).unwrap();
    }
    info!("creating new files directory at path={:?}", CONFIG.root_dir);
    _ = fs::create_dir(&CONFIG.root_dir);
}

#[tokio::main]
async fn main() {
    setup_env();
    setup_tracing();
    setup_files_dir();
    info!("config = {CONFIG:?}");
    // app state
    let state = Arc::new(RunTimeState::new().await);

    // axum app
    let app = Router::new()
        .route("/", get(root).post(download))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err: BoxError| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled error: {}", err),
                    )
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(
                    CONFIG.requests_per_minute,
                    Duration::from_mins(1),
                )),
        )
        .layer(
            TraceLayer::new_for_http()
                .on_request(|_request: &Request<_>, _span: &Span| {
                    tracing::debug!("new request -------------------")
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    tracing::debug!("response done -----------------")
                })
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        tracing::error!("request failed")
                    },
                ),
        )
        .with_state(state);

    // serve
    let addr = SocketAddr::new(CONFIG.internal_host.parse().unwrap(), CONFIG.internal_port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("listening at addr={addr}");
    axum::serve(listener, app).await.unwrap();
}
