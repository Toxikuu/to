// utils/log.rs

use std::path::Path;
use std::fs::{self, File};
use std::io::{self, BufWriter, BufReader, BufRead, Write};
use std::collections::VecDeque;
use std::str::FromStr;
use tempfile::NamedTempFile;
use tracing::{error, debug};
use std::sync::OnceLock;

use tracing::{
    level_filters::LevelFilter,
};

use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling,
};
use tracing_subscriber::{
    EnvFilter,
    fmt::time,
    prelude::*,
};

use crate::CONFIG;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
const LOG_FILE: &str = "/var/log/to.log";

/// # Trims a log file until it's under a maximum size
///
/// Trimming means deleting lines from the top of the file
///
/// # Arguments
/// * `path`        - The path to the log file to be trimmed
/// * `max_size`    - The maximum size of the log file, in bytes
///
/// # Returns
/// Bytes trimmed
///
/// # Errors
/// - Log file does not exist (`NotFound` should be handled when called)
/// - Other I/O errors or something
///
/// # Examples
/// ```rust
/// const MAX_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB
/// trim_log("hello.log", MAX_SIZE).permit(|e| e.kind() == std::io::ErrorKind::NotFound)
/// ```
pub fn trim_log<P: AsRef<Path>>(path: P, max_size: u64) -> io::Result<u64> {
    let path = path.as_ref();
    let size = fs::metadata(path)?.len();

    if size <= max_size {
        // dbug!("Log size is {size}");
        return Ok(0);
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = VecDeque::new();
    let mut total_size = 0;

    for line in reader.lines() {
        let line = line?;
        let line_size = (line.len() + 1) as u64; // account for \n

        total_size += line_size;
        lines.push_back((line, line_size));

        while total_size > max_size {
            if let Some((_, removed_size)) = lines.pop_front() {
                total_size -= removed_size;
            }
        }
    }

    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = BufWriter::new(&mut temp_file);
        for (line, _) in &lines {
            writeln!(writer, "{line}")?;
        }
    }

    temp_file.persist(path)?;
    Ok(size - total_size)
}

pub fn log() {
    let file_appender = {
        let (dir, file) = LOG_FILE.rsplit_once('/').unwrap();
        rolling::never(dir, file)
    };

    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let level = LevelFilter::from_str(&CONFIG.log_level).unwrap_or(LevelFilter::DEBUG);
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .with_env_var("LOG_LEVEL")
        .from_env_lossy()
        // Silence some loud crates
        .add_directive("dircpy=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("fshelpers=warn".parse().unwrap())
        .add_directive("hyper_util=warn".parse().unwrap());

    if CONFIG.log_to_console {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_timer(time::uptime())
            .with_writer(file_writer.and(io::stdout))
            .compact()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_timer(time::uptime())
            .with_writer(file_writer)
            .compact()
            .init();
    }

    if LOG_GUARD.set(guard).is_err() {
        eprintln!("The log() function was called more than once.");
        eprintln!("Please report this as a bug.");
    }
}

/// # Initialize logging
///
/// This function wraps all the logging setup, including trimming
pub fn init() {
    log();
    match trim_log(LOG_FILE, CONFIG.log_max_size) {
        Ok(b) => debug!("Trimmed {b} bytes from log file"),
        Err(e) => error!("Failed to trim bytes from log file: {e}"),
    }
}
