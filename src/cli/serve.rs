use clap::Args;

use super::CommandError;
use crate::server;

#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    // TODO: Remove this allow once I add arguments for serve
    #[allow(unused_variables)]
    pub async fn run(&self) -> Result<(), CommandError> { Ok(server::core::serve().await?) }
}
