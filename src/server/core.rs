// server/core.rs

use axum::{
    Router,
    routing::{
        get,
        post,
    },
};
use tracing::info;

use super::endpoints::*;

pub const ADDR: &str = "127.0.0.1:7020";

pub async fn serve() -> anyhow::Result<()> {
    let router = Router::new()
        .route("/{filename}", get(download))
        .route("/up/{filename}", post(upload));

    let listener = tokio::net::TcpListener::bind(ADDR).await?;
    axum::serve(listener, router).await?;

    info!("Listening on {ADDR}");
    Ok(())
}
