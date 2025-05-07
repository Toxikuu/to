// server/endpoints.rs

use std::path::PathBuf;

use axum::{
    body::Body,
    extract::Path,
    http::{
        HeaderMap,
        StatusCode,
        header,
    },
    response::IntoResponse,
};
use futures::StreamExt;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
};
use tokio_util::io::ReaderStream;
use tracing::{
    debug,
    error,
    warn,
};

const DIST: &str = "/srv/to/dist";

pub async fn download(Path(filename): Path<String>) -> impl IntoResponse {
    let path = PathBuf::from(DIST).join(&filename);

    match File::open(&path).await {
        | Ok(file) => {
            let mut headers = HeaderMap::new();
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            // Add mime type header
            let mime = mime_guess::from_path(&filename).first_or_octet_stream();
            headers.insert(header::CONTENT_TYPE, mime.to_string().parse().unwrap());

            // Add content length header
            if let Ok(meta) = tokio::fs::metadata(&path).await {
                headers.insert(
                    header::CONTENT_LENGTH,
                    meta.len().to_string().parse().unwrap(),
                );
            }

            // Add content disposition header
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename)
                    .parse()
                    .unwrap(),
            );

            (headers, body).into_response()
        },
        | Err(e) => {
            warn!("{e}");
            StatusCode::NOT_FOUND.into_response()
        },
    }
}

pub async fn upload(Path(filename): Path<String>, body: Body) -> impl IntoResponse {
    let path = PathBuf::from(DIST).join(&filename);
    let mut file = match File::create(&path).await {
        | Ok(f) => f,
        | Err(e) => {
            error!("Failed to create file: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        },
    };

    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            | Ok(bytes) => {
                if let Err(e) = file.write_all(&bytes).await {
                    eprintln!("Write error: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            },
            | Err(e) => {
                eprintln!("Body error: {e}");
                return StatusCode::BAD_REQUEST;
            },
        }
    }

    debug!("Uploaded {filename}");
    StatusCode::OK
}
