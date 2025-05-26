// server/endpoints.rs

use std::{
    io::{
        self,
        ErrorKind,
    },
    path::{
        Path,
        PathBuf,
    },
};

use axum::{
    body::Body,
    extract::Path as Apath,
    http::{
        HeaderMap,
        HeaderValue,
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

pub async fn download(Apath(filename): Apath<String>) -> impl IntoResponse {
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

            // Add last modified header
            if let Ok(modtime) = generate_last_modified_header(&path).await {
                headers.insert("Last-Modified", modtime);
            }

            // Add content disposition header
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\"")
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

pub async fn upload(Apath(filename): Apath<String>, body: Body) -> impl IntoResponse {
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

async fn generate_last_modified_header<P: AsRef<Path>>(path: P) -> Result<HeaderValue, io::Error> {
    let metadata = tokio::fs::metadata(path.as_ref()).await?;
    let modtime = metadata.modified()?;
    let datetime = httpdate::fmt_http_date(modtime);
    Ok(datetime.parse().map_err(|_| ErrorKind::InvalidData)?)
}
