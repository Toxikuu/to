// package/pull.rs
//! Code related to pulling packages

use std::{
    fs::{
        File,
        rename,
    },
    io::Write,
    path::Path,
    time::Duration,
};

use anyhow::{
    Context,
    Result,
    bail,
};
use fshelpers::mkdir_p;
use futures::{
    StreamExt,
    future::join_all,
};
use indicatif::{
    MultiProgress,
    ProgressBar,
    ProgressStyle,
};
use permitit::Permit;
use reqwest::{
    Client,
    header::{
        HeaderMap,
        USER_AGENT,
    },
    redirect::Policy,
};
use tokio::task;
use tracing::{
    debug,
    error,
    warn,
};

use super::Package;
use crate::structs::config::CONFIG;

pub async fn multipull(pkgs: &[Package]) -> Result<()> {
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
            .context("Nameless distfile")?
            .to_string_lossy()
            .to_string();
        let url = format!("http://{addr}/{filename}");

        // set up progress bar
        let pb = m.add(ProgressBar::new(0));
        pb.set_style(sty.clone());
        pb.set_message(format!("{pkg:-}"));
        pb.set_prefix("\x1b[37;1m[\x1b[36mo\x1b[37m]\x1b[0m");
        pb.set_position(0);
        pb.set_length(1);
        pb.tick();

        let task = task::spawn(async move {
            match download_file(client, &url, &distfile, pb.clone())
                .await
                .permit(|e| e.to_string() == "extant")
            {
                | Ok(()) => pb.set_prefix("\x1b[37;1m[\x1b[32m*\x1b[37m]\x1b[0m"),
                | Err(e) => {
                    pb.set_prefix("\x1b[37;1m[\x1b[31m-\x1b[37m]\x1b[0m");
                    error!("Failed to download {filename} from {url}: {e}");
                },
            }
        });
        tasks.push(task);
    }

    join_all(tasks).await;
    Ok(())
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
async fn download_file<P>(client: Client, url: &str, filename: P, pb: ProgressBar) -> Result<()>
where
    P: AsRef<Path>,
{
    let filename = filename.as_ref();
    let filename_display = filename.display();
    debug!("Downloading '{url}' to '{filename_display}'");

    // it'll be manually updated
    pb.disable_steady_tick();

    // skip extant files
    if filename.exists() {
        let file_size = filename.metadata().map(|f| f.len()).unwrap_or(0);
        pb.set_length(file_size);
        pb.set_position(file_size);
        pb.finish();
        pb.tick();
        debug!("Skipping download for extant file {filename_display}");
        bail!("extant");
    }

    // fetch the url
    debug!("Fetching url '{url}'");
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to get a response from {url}"))?;
    let content_length = resp.content_length();
    debug_assert!(content_length.map(|l| l > 0).unwrap_or(true));
    debug!("Content length reported as {content_length:?}");

    // create a part file
    let partfile_ = filename.with_added_extension("part");
    let mut partfile = File::create(&partfile_)?;
    let mut stream = resp.bytes_stream();

    // set the download size if known
    if let Some(size) = content_length {
        pb.set_length(size);
        pb.tick()
    }

    // write the file and set the progress bar length
    let mut downloaded = 0;
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        partfile.write_all(&data)?;
        downloaded += data.len() as u64;

        if content_length.is_none() {
            pb.set_length(downloaded);
        }
        pb.set_position(downloaded);
        pb.tick();
    }

    // move the part file to the final destination
    rename(partfile_, filename)?;
    pb.finish();
    pb.tick();
    debug!("Downloaded '{url}' to '{filename_display}'");

    Ok(())
}

/// # Create a reqwest client
///
/// This client follows redirects up to 16 times
/// It sets a user agent of "to/0.0.0"
/// It timeouts after 32 seconds
/// It ignores invalid http1 headers
async fn create_client() -> std::result::Result<Client, reqwest::Error> {
    let client = Client::builder()
        .redirect(Policy::limited(16))
        .http1_ignore_invalid_headers_in_responses(true)
        .default_headers({
            let mut headers = HeaderMap::new();
            headers.insert(USER_AGENT, "to/0.1.0".parse().unwrap());
            headers
        })
        .timeout(Duration::from_secs(32))
        .build();
    if client.is_err() {
        error!("Failed to build http client");
        warn!("Ensure /etc/protocols, /etc/resolv.conf, and /etc/nsswitch.conf are sane");
    }
    client
}

/// # Initial setup for the client and progress bar
pub async fn setup() -> Result<(Client, MultiProgress, ProgressStyle)> {
    let client = create_client().await.context("Failed to create client")?;
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "{prefix} {msg:<36} [{bar:18.red/black}] {decimal_total_bytes}",
    )
    .unwrap()
    .progress_chars("=> ");
    Ok((client, m, sty))
}
