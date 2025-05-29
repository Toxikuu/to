// utils/exec.rs
//! Utility functions for command execution

// TODO: Probably use BASH_ENV= to avoid bash just fucking ignoring my whole environment.

use std::{
    io::{
        self,
        BufRead,
    },
    process::{
        Command,
        Stdio,
    },
    thread,
};

use tracing::{
    debug,
    error,
    trace,
};

use crate::config::CONFIG;

pub fn sex(command: &str) -> io::Result<String> {
    let command = prepend_source_base(command);

    let output = Command::new("bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-e")
        .arg("-c")
        .arg(&command)
        .output()?;
    debug!("Statically executing {command}");

    let status = output.status;
    if !status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("Command `{command}` returned error:\n{error}");
        return Err(io::Error::other(format!(
            "Command failed with status: {status}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    trace!("Received output:\n{stdout}");
    Ok(stdout)
}

pub fn exec(command: &str) -> io::Result<()> {
    let command = prepend_source_base(command);
    let command_clone = command.clone();

    let mut child = Command::new("bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-e")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_thread = thread::spawn(move || {
        let reader = io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                | Ok(line) => {
                    trace!(" [STDOUT] {line}");
                },
                | Err(e) => error!("Error reading stdout: {e}"),
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        let reader = io::BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                | Ok(line) => {
                    debug!(" [STDERR] {line}");
                },
                | Err(e) => error!("Error reading stderr: {e}"),
            }
        }
    });

    let status = child.wait()?;
    if !status.success() {
        error!("Command '{command_clone}' failed with status {status}");
        return Err(io::Error::other(format!(
            "Command failed with status: {status}"
        )));
    }

    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();

    Ok(())
}

pub fn exec_interactive(command: &str) -> io::Result<()> {
    let command = prepend_source_base(command);

    let status = Command::new("bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-e")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Command failed with status: {status}"
        )));
    }

    Ok(())
}

fn prepend_source_base(command: &str) -> String {
    format!(
        r#"TO_CFLAGS="{}" TO_JOBS="{}" source /usr/share/to/envs/base.env ; {command}"#,
        CONFIG.cflags, CONFIG.jobs
    )
}

#[macro_export]
macro_rules! exec {
    ($($cmd:tt)*) => {{
        $crate::utils::exec::exec(&format!($($cmd)*))
    }};
}

#[macro_export]
macro_rules! exec_interactive {
    ($($cmd:tt)*) => {{
        $crate::utils::exec::exec_interactive(&format!($($cmd)*))
    }};
}

#[macro_export]
macro_rules! sex {
    ($($cmd:tt)*) => {{
        $crate::utils::exec::sex(&format!($($cmd)*))
    }};
}
