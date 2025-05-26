// package/message.rs
//! Code related to package messages

use std::{
    fmt,
    fs::read_to_string,
    path::{
        Path,
        PathBuf,
    },
    str::FromStr,
};

use thiserror::Error;
use tracing::{
    debug,
    error,
    trace,
};

use super::Package;
use crate::structs::cli::{
    CLI,
    SubCommand,
};

/// # A message for a package
/// Messages contain a title, contents, and a hook. They are tied to a package, though the package
/// isn't part of the struct.
#[derive(Debug)]
pub struct Message {
    title:   String,
    content: String,
    hook:    MessageHook,
}

/// # A hook for when a message should be displayed
#[derive(Debug, PartialEq)]
pub enum MessageHook {
    Install,
    Remove,
    Update,
}

#[derive(Debug, Error)]
#[error("Invalid message hook: {0}")]
pub struct ParseMessageHookError(String);

impl FromStr for MessageHook {
    type Err = ParseMessageHookError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            | "install" => Ok(MessageHook::Install),
            | "remove" => Ok(MessageHook::Remove),
            | "update" => Ok(MessageHook::Update),
            | _ => Err(ParseMessageHookError(s.to_owned())),
        }
    }
}

impl fmt::Display for MessageHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            | MessageHook::Install => "Install",
            | MessageHook::Remove => "Remove",
            | MessageHook::Update => "Update",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("File does not exist: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid filename")]
    InvalidFilename,

    #[error("Invalid UTF-8 in filename")]
    InvalidTitleEncoding,

    #[error("Failed to read file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse hook: {0}")]
    ParseHook(#[from] ParseMessageHookError),
}

impl Message {
    /// # Collect all messages from a file in the message directory
    /// An m file can contain multiple "messages". These scare-quoted "messages" are all a single
    /// message in that they have the same title, but have different hooks.
    ///
    /// For possible errors, view the branches of the MessageError enum.
    fn from_m_file<P>(path: P) -> Result<Vec<Self>, MessageError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if !path.exists() {
            return Err(MessageError::FileNotFound(path.to_path_buf()));
        }

        let title = path
            .file_name()
            .ok_or(MessageError::InvalidFilename)?
            .to_str()
            .ok_or(MessageError::InvalidTitleEncoding)?
            .to_string();

        let raw = read_to_string(path)?;
        let raw_lines = raw.lines().collect::<Vec<_>>();
        trace!("Received '{raw}' from mfile {path:?}");

        // Trim out an empty line if it precedes a hook
        let mut lines = Vec::new();
        let mut i = 0;
        while i < raw_lines.len() {
            if raw_lines[i].trim().is_empty()
                && i + 1 < raw_lines.len()
                && raw_lines[i + 1].starts_with(",hook ")
            {
                // Skip the line
                i += 1;
                continue;
            }

            lines.push(raw_lines[i]);
            i += 1;
        }

        let mut messages = Vec::new();
        let mut current_hook: Option<MessageHook> = None;
        let mut current_content = String::new();

        for line in lines {
            // Each message needs a hook, so a new message occurs whenever a hook is defined
            if let Some(hook_str) = line.strip_prefix(",hook ") {
                // Add the previous message, if there was one
                if let Some(hook) = current_hook.take() {
                    messages.push(Self {
                        hook,
                        title: title.clone(),
                        content: std::mem::take(&mut current_content),
                    });
                }

                // Start a new message
                current_hook = Some(hook_str.parse()?);
            } else if current_hook.is_some() {
                // Append content to the current message
                current_content.push_str(line);
                // BUG: \n should not be appended for the final content line
                current_content.push('\n');
            }
        }

        // EOF case
        if let Some(hook) = current_hook {
            messages.push(Self {
                hook,
                title,
                content: current_content,
            });
        }

        Ok(messages)
    }

    pub fn display(&self) {
        println!("\x1b[3m -=- {} -=- \x1b[0m", self.title);
        println!("\x1b[36m{}\x1b[0m", format_message_content(&self.content));
    }

    pub fn display_with_hook(&self) {
        println!(
            "\x1b[3m -=- {} -=- \x1b[34m({})\x1b[0m",
            self.title, self.hook
        );
        println!("\x1b[36m{}\x1b[0m", format_message_content(&self.content));
    }
}

fn format_message_content(content: &str) -> String {
    content
        .lines()
        .map(|l| {
            if l.starts_with(" $ ") {
                format!("\x1b[36;1m{l}\x1b[0m\n")
            } else {
                format!("{l}\n")
            }
        })
        .collect::<String>()
}

impl Package {
    /// # Returns the message directory of a package
    /// This directory is defined as /var/db/to/pkgs/<package>/M
    pub fn messagedir(&self) -> PathBuf { self.pkgfile().with_file_name("M") }

    /// # Collects all messages for a package
    /// Returns an empty vector if a package has nome
    fn collect_messages(&self) -> Vec<Message> {
        let messagedir = &self.messagedir();
        trace!(
            "Collecting messages from {} for {self:-}",
            messagedir.display()
        );
        if !messagedir.is_dir() {
            // TODO: If I ever wanna be really thorough, I might should log the case where
            // messagedir exists but isn't a dir.
            trace!("No messages exist for {self:-}");
            return vec![];
        }

        let Ok(entries) = messagedir
            .read_dir()
            .inspect_err(|e| error!("Failed to get messages for {self:-}: {e}"))
        else {
            return vec![];
        };

        let mut messages = vec![];
        for entry in entries.map_while(Result::ok) {
            let mfile = entry.path();
            let mut message = Message::from_m_file(&mfile)
                .inspect_err(|e| {
                    error!(
                        "Failed to get some messages for {self} from {}: {e}",
                        mfile.display()
                    )
                })
                .unwrap_or_default();

            debug!("Generated message {message:#?} from {mfile:?}");
            messages.append(&mut message);
        }

        messages
    }

    /// # Displays messages for a package
    pub fn message(&self, hook: MessageHook) {
        if let SubCommand::Remove(args) = &*CLI {
            if args.suppress_messages {
                debug!("Suppressing messages for {self}");
                return;
            }
        }

        let messages = self.collect_messages();
        let messages = messages
            .iter()
            .filter(|m| m.hook == hook)
            .collect::<Vec<_>>();

        match messages.len() {
            | 0 => {}, // no messages
            | 1 => {
                println!("\x1b[36;1mMessage from {self}:\x1b[0m");
                if let [only] = messages.as_slice() {
                    only.display();
                }
            },
            | _ => {
                // multiple messages
                println!("\x1b[36;1mMessages from {self}:\x1b[0m");
                messages.iter().for_each(|m| m.display());
            },
        }
    }

    pub fn view_all_messages(&self, quiet: bool) {
        let messages = self.collect_messages();

        match messages.len() {
            | 0 => {
                if !quiet {
                    println!("\x1b[36;1mNo messages for {self}\x1b[0m")
                }
            },
            | 1 => {
                println!("\x1b[36;1mMessage from {self}:\x1b[0m");
                if let [only] = messages.as_slice() {
                    only.display();
                }
            },
            | _ => {
                println!("\x1b[36;1mMessages from {self}:\x1b[0m");
                messages.iter().for_each(|m| m.display_with_hook());
            },
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        path::Path,
        process::{
            ExitCode,
            Termination,
        },
    };

    use super::*;

    // Don't mind the cursed test "skipping" setup
    // Stolen and adapted from https://plume.benboeckel.net/~/JustAnotherBlog/skipping-tests-in-rust

    #[derive(Debug)]
    #[allow(dead_code)] // Used to display skip messages for tests, even though it's not "used"
    struct Skip(&'static str);

    impl Termination for Skip {
        fn report(self) -> ExitCode { 77.into() }
    }

    macro_rules! skip {
        ($reason:expr) => {
            return Err(Skip($reason))
        };
    }

    #[test]
    fn test_pulseaudio_messages() -> Result<(), Skip> {
        let pkgdir = Path::new("/var/db/to/pkgs");

        if !pkgdir.exists() {
            skip!("Missing package directory")
        }

        if !pkgdir.join("pulseaudio").exists() {
            skip!("Missing package 'pulseaudio'")
        }

        let pkg = Package::from_s_file("pulseaudio").unwrap();

        assert!(pkg.messagedir() == PathBuf::from("/var/db/to/pkgs/pulseaudio/M"));
        assert!(!pkg.collect_messages().is_empty());
        Ok(())
    }
}
