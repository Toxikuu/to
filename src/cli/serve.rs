use clap::Args;

use color_eyre::Result as Eresult;
use crate::{
    CONFIG,
    server,
};

#[derive(Args, Debug)]
pub struct Command {
    /// The address to which the server should bind
    ///
    /// For example, http://127.0.0.1:7020
    #[arg(short, long)]
    pub addr: Option<String>,
}

/// Run a distfile server
impl Command {
    pub async fn run(&self) -> Eresult<()> {
        let full_addr = self.addr.as_ref().unwrap_or(&CONFIG.server_address);
        Ok(server::core::serve(full_addr).await?)
    }
}
