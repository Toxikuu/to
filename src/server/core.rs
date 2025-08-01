// server/core.rs

use std::io;

use axum::{
    Router,
    routing::{
        get,
        post,
    },
};
use thiserror::Error;
use tracing::info;

use super::endpoints::*;

#[derive(Debug, Error)]
pub enum ServeError {
    #[error("Failed to bind to server address")]
    Bind(#[from] io::Error),
}

pub async fn serve(full_addr: &str) -> Result<(), ServeError> {
    let router = Router::new()
        .route("/{filename}", get(download))
        .route("/up/{filename}", post(upload));

    let addr = full_addr
        .split_once("://")
        .map(|a| a.1)
        .unwrap_or(full_addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Allegedly, this never returns an error:
    // "Although this future resolves to `io::Result<()>`, it will never actually complete or
    // return an error. Errors on the TCP socket will be handled by sleeping for a short while
    // (currently, one second)."
    // -- Axum API documentation
    axum::serve(listener, router).await.unwrap();
    info!("Serving on {full_addr}");
    Ok(())
}
