// package/pull.rs
//! Code related to pulling packages

use std::{
    fs::{
        File,
        rename,
    },
    io::{
        ErrorKind,
        Write,
    },
    path::Path,
    time::{
        Duration,
        SystemTime,
    },
};

use fshelpers::mkdir_p;
use futures::{
    StreamExt,
    future::join_all,
    io,
};
use indicatif::{
    MultiProgress,
    ProgressBar,
    ProgressStyle,
};
use reqwest::{
    Client,
    Response,
    header::{
        HeaderMap,
        LAST_MODIFIED,
        USER_AGENT,
    },
    redirect::Policy,
};
use thiserror::Error;
use tokio::task;
use tracing::{
    debug,
    error,
    trace,
    warn,
};

use super::Package;
use crate::config::CONFIG;

pub fn get_upstream_modtime(headers: &HeaderMap) -> Option<SystemTime> {
    let h = headers.get(LAST_MODIFIED)?;
    let s = h.to_str().ok()?;
    let t = httpdate::parse_http_date(s).ok()?;
    Some(t)
}

pub fn get_local_modtime(path: &Path) -> Option<SystemTime> {
    let m = path.metadata().ok()?;
    let t = m.modified().ok()?;
    Some(t)
}

#[derive(Debug, Error)]
pub enum DownloadError {
    // #[error("File exists and is current")]
    // Extant,
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Couldn't create client: {0}")]
    CreateClient(reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to get server modtime")]
    GetServerModtime,
}

pub async fn multipull(pkgs: &[Package], force: bool) -> Result<(), DownloadError> {
    let addr = &CONFIG.server_address;
    let (client, m, sty) = setup().await?;
    let mut tasks = Vec::new();

    // distfile contains the full path here
    for pkg in pkgs {
        let distfile = pkg.distfile();
        let distdir = pkg.distdir();
        mkdir_p(distdir)?;

        let client = client.clone();
        let filename = distfile
            .file_name()
            .ok_or(io::Error::from(ErrorKind::InvalidFilename))?
            .to_string_lossy()
            .to_string();
        let url = format!("{addr}/{filename}");

        let m = m.clone();
        let sty = sty.clone();
        let msg = format!("{pkg:-}");
        let task = task::spawn(async move {
            match should_download(&client, &url, &distfile, force).await {
                | Ok(Some(r)) => {
                    // set up progress bar
                    let pb = m.add(ProgressBar::new(0));
                    pb.set_style(sty.clone());
                    pb.set_message(msg);
                    pb.set_prefix("\x1b[37;1m[\x1b[36mo\x1b[37m]\x1b[0m");
                    pb.set_position(0);
                    pb.set_length(1);
                    pb.tick();

                    match download_file(r, &distfile, pb.clone()).await {
                        | Ok(()) => pb.set_prefix("\x1b[37;1m[\x1b[32m*\x1b[37m]\x1b[0m"),
                        | Err(e) => {
                            pb.set_prefix("\x1b[37;1m[\x1b[31m-\x1b[37m]\x1b[0m");
                            error!("Failed to download {filename} from {url}: {e}");
                        },
                    }
                },
                | Ok(None) => {
                    debug!("Skipping {msg}");
                },
                | Err(e) => {
                    error!("Error checking whether {filename} should be downloaded: {e}")
                },
            }
        });
        tasks.push(task);
    }

    join_all(tasks).await;
    Ok(())
}

async fn should_download(
    client: &Client,
    url: &str,
    file: &Path,
    force: bool,
) -> Result<Option<Response>, DownloadError> {
    // debug!("Checking whether {url} should be downloaded");

    let resp = client.get(url).send().await?.error_for_status()?;
    let headers = resp.headers();

    if force {
        // debug!("Should download because --force was passed");
        return Ok(Some(resp))
    }

    if !file.exists() {
        // debug!("Should download because local file does not exist");
        return Ok(Some(resp))
    }

    let server_modtime = get_upstream_modtime(headers).ok_or(DownloadError::GetServerModtime)?;
    let local_modtime = get_local_modtime(file).unwrap_or(SystemTime::UNIX_EPOCH);

    if server_modtime <= local_modtime {
        // debug!(
        //     "Should not download because local file ({local_modtime:?}) is not older than server's {server_modtime:?}"
        // );
        Ok(None)
    } else {
        // debug!(
        //     "Should download because server's file ({server_modtime:?}) is newer than local ({local_modtime:?})"
        // );
        Ok(Some(resp))
    }
}

/// # Download a file
///
/// This function takes a url and filename. The filename can be a path.
/// It also takes a reqwest client, and an indicatif progressbar.
///
/// Extant files return the io error 'AlreadyExists', which should be permitted if desired.
///
/// It also writes to a part file before moving to the destination on completion. Note that
/// downloads cannot be resumed because that shit hurts my head.
async fn download_file<P>(resp: Response, filename: P, pb: ProgressBar) -> Result<(), DownloadError>
where
    P: AsRef<Path>,
{
    let filename = filename.as_ref();
    let filename_display = filename.display();
    debug!("Downloading '{filename_display}'");

    // it'll be manually updated
    pb.disable_steady_tick();

    // reuse the response we fetched earlier
    let resp = resp.error_for_status()?;
    let headers = resp.headers();
    dbg!(&headers);

    let server_modtime = match get_upstream_modtime(headers) {
        | Some(t) => t,
        | None => {
            error!("Failed to get server modtime for {}", filename.display());
            return Err(DownloadError::GetServerModtime);
        },
    };

    // NOTE: `resp.content_lenght()` fails for head().
    let content_length = resp.content_length().unwrap_or(0);
    debug_assert!(content_length > 0);

    // create a part file
    let partfile_ = filename.with_added_extension("part");
    let mut partfile = File::create(&partfile_)?;

    let mut stream = resp.bytes_stream();
    trace!("Created partfile at {partfile:?}");

    // set the download size if known
    pb.set_length(content_length);
    pb.tick();

    // write the file and set the progress bar length
    let mut downloaded = 0;
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        partfile.write_all(&data)?;
        downloaded += data.len() as u64;

        pb.set_position(downloaded);
        pb.tick();
    }

    // move the part file to the final destination and properly set its modtime
    rename(partfile_, filename)?;
    filetime::set_file_mtime(
        filename,
        filetime::FileTime::from_system_time(server_modtime),
    )?;

    pb.finish();
    pb.tick();
    debug!("Downloaded '{filename_display}'");

    Ok(())
}

/// # Create a reqwest client
///
/// This client follows redirects up to 16 times
/// It sets a user agent of "to/0.0.0"
/// It timeouts after 32 seconds
/// It ignores invalid http1 headers
pub async fn create_client() -> Result<Client, reqwest::Error> {
    let client = Client::builder()
        .redirect(Policy::limited(16))
        .http1_ignore_invalid_headers_in_responses(true)
        .default_headers({
            let mut headers = HeaderMap::new();
            headers.insert(USER_AGENT, "to/0.1.0".parse().unwrap());
            headers
        })
        .connect_timeout(Duration::from_secs(32))
        .build();
    if client.is_err() {
        error!("Failed to build http client");
        warn!("Ensure /etc/protocols, /etc/resolv.conf, and /etc/nsswitch.conf are sane");
    }
    client
}

/// # Initial setup for the client and progress bar
pub async fn setup() -> Result<(Client, MultiProgress, ProgressStyle), DownloadError> {
    let client = create_client().await.map_err(DownloadError::CreateClient)?;
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "{prefix} {msg:<36} [{bar:18.red/black}] {decimal_total_bytes}",
    )
    .unwrap()
    .progress_chars("=> ");
    Ok((client, m, sty))
}
