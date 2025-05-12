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
use crate::structs::config::CONFIG;

pub async fn serve() -> anyhow::Result<()> {
    let router = Router::new()
        .route("/{filename}", get(download))
        .route("/up/{filename}", post(upload));

    let addr = &CONFIG.server_address;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    info!("Listening on {addr}");
    Ok(())
}
